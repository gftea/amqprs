use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use amqp_serde::types::AmqpChannelId;
use tokio::sync::{mpsc, oneshot, broadcast, Notify};

use crate::frame::{
    Close, Frame, Open, OpenChannel, ProtocolHeader, StartOk, TuneOk, CONN_DEFAULT_CHANNEL,
};
use crate::{
    frame::{Ack, BasicProperties, Deliver},
    net::{
        self, ChannelResource, IncomingMessage, ManagementCommand, OutgoingMessage, SplitConnection,
    },
};

use super::{error::Error, channel::{Acker, SharedConsumerQueue}};
use super::{channel::Channel, consumer::Consumer};
type Result<T> = std::result::Result<T, Error>;

/////////////////////////////////////////////////////////////////////////////

/////////////////////////////////////////////////////////////////////////////
pub struct ClientCapabilities {}
pub struct ServerCapabilities {}
pub struct Connection {
    capabilities: Option<ServerCapabilities>,
    is_open: bool,
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    incoming_rx: mpsc::Receiver<IncomingMessage>,
    mgmt_tx: mpsc::Sender<ManagementCommand>,
}
//  TODO: move below constants gto be part of static configuration of connection
const DISPATCHER_FRAME_BUFFER_SIZE: usize = 256;
const DISPATCHER_COMMAND_BUFFER_SIZE: usize = 32;

const INCOMING_RESPONSE_BUFFER_SIZE: usize = 32;

const OUTGOING_MESSAGE_CHANNEL_BUFFER_SIZE: usize = 128;
const MANAGEMENT_CHANNEL_BUFFER_SIZE: usize = 32;
/// AMQP Connection API
///
impl Connection {
    /// Open a AMQP connection
    pub async fn open(uri: &str) -> Result<Self> {
        // TODO: uri parsing
        let mut connection = SplitConnection::open(uri).await?;

        // TODO: protocol header negotiation ?
        connection.write(&ProtocolHeader::default()).await?;

        // S: 'Start'
        let (_, frame) = connection.read_frame().await?;
        get_expected_method!(
            frame,
            Frame::Start,
            Error::ConnectionOpenError("start".to_string())
        )?;

        // C: 'StartOk'
        let start_ok = StartOk::default().into_frame();
        connection
            .write_frame(CONN_DEFAULT_CHANNEL, start_ok)
            .await?;

        // S: 'Tune'
        let (_, frame) = connection.read_frame().await?;
        let tune = get_expected_method!(
            frame,
            Frame::Tune,
            Error::ConnectionOpenError("tune".to_string())
        )?;
        // C: TuneOk
        let mut tune_ok = TuneOk::default();
        tune_ok.channel_max = tune.channel_max;
        tune_ok.frame_max = tune.frame_max;
        tune_ok.heartbeat = tune.heartbeat;

        let channel_max = tune_ok.channel_max;
        let _heartbeat = tune_ok.channel_max;
        connection
            .write_frame(CONN_DEFAULT_CHANNEL, tune_ok.into_frame())
            .await?;

        // C: Open
        let open = Open::default().into_frame();
        connection.write_frame(CONN_DEFAULT_CHANNEL, open).await?;

        // S: OpenOk
        let (_, frame) = connection.read_frame().await?;
        get_expected_method!(
            frame,
            Frame::OpenOk,
            Error::ConnectionOpenError("open".to_string())
        )?;

        // spawn network management tasks and get internal channel' sender half.
        let (outgoing_tx, outgoing_rx) = mpsc::channel(OUTGOING_MESSAGE_CHANNEL_BUFFER_SIZE);
        let (mgmt_tx, mgmt_rx) = mpsc::channel(MANAGEMENT_CHANNEL_BUFFER_SIZE);

        net::spawn_handlers(
            connection,
            channel_max,
            outgoing_tx.clone(),
            outgoing_rx,
            mgmt_tx.clone(),
            mgmt_rx,
        )
        .await;

        let (responder, incoming_rx) = mpsc::channel(INCOMING_RESPONSE_BUFFER_SIZE);

        net::register_channel_resource(
            &mgmt_tx,
            Some(CONN_DEFAULT_CHANNEL),
            ChannelResource {
                responder,
                dispatcher: None,
            },
        )
        .await
        .ok_or_else(|| {
            Error::ConnectionOpenError("register channel resource failure".to_string())
        })?;

        Ok(Self {
            capabilities: None,
            is_open: true,
            outgoing_tx,
            incoming_rx,
            mgmt_tx,
        })
    }

    /// close and consume the AMQ connection
    pub async fn close(mut self) -> Result<()> {
        synchronous_request!(
            self.outgoing_tx,
            (CONN_DEFAULT_CHANNEL, Close::default().into_frame()),
            self.incoming_rx,
            Frame::CloseOk,
            Error::ConnectionCloseError
        )?;
        self.is_open = false;
        Ok(())
    }

    /// open a AMQ channel
    pub async fn open_channel(&self) -> Result<Channel> {
        let (responder, incoming_rx) = mpsc::channel(INCOMING_RESPONSE_BUFFER_SIZE);
        let (dispatcher, dispatcher_rx) = mpsc::channel(DISPATCHER_FRAME_BUFFER_SIZE);
        let (dispatcher_mgmt_tx, dispatcher_mgmt_rx) = mpsc::channel(DISPATCHER_COMMAND_BUFFER_SIZE);

        let channel_id = net::register_channel_resource(
            &self.mgmt_tx,
            None,
            ChannelResource {
                responder,
                dispatcher: Some(dispatcher),
            },
        )
        .await
        .ok_or_else(|| {
            Error::ConnectionOpenError("register channel resource failure".to_string())
        })?;

        let consumer_queue: SharedConsumerQueue = Arc::new(Mutex::new(BTreeMap::new()));
        let park_notify = Arc::new(Notify::new());
        let unpark_notify = Arc::new(Notify::new());

        let mut channel = Channel {
            is_open: false,
            channel_id,
            outgoing_tx: self.outgoing_tx.clone(),
            incoming_rx,
            mgmt_tx: self.mgmt_tx.clone(),
            dispatcher_rx: Some(dispatcher_rx),
            dispatcher_mgmt_tx,
            dispatcher_mgmt_rx: Some(dispatcher_mgmt_rx),
        };
        synchronous_request!(
            channel.outgoing_tx,
            (channel.channel_id, OpenChannel::default().into_frame()),
            channel.incoming_rx,
            Frame::OpenChannelOk,
            Error::ChannelOpenError
        )?;
        channel.is_open = true;
        channel.spawn_dispatcher().await;

        Ok(channel)
    }



}

impl Drop for Connection {
    fn drop(&mut self) {
        if self.is_open {
            let tx = self.outgoing_tx.clone();
            let _handle = tokio::spawn(async move {
                tx.send((CONN_DEFAULT_CHANNEL, Close::default().into_frame()))
                    .await
                    .unwrap();
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;
    use tokio::time;

    #[tokio::test]
    async fn test_channel_open_use_close() {
        {
            // test close on drop
            let client = Connection::open("localhost:5672").await.unwrap();

            {
                // test close on drop
                let _channel = client.open_channel().await.unwrap();
                // channel.close().await.unwrap();
            }
            time::sleep(time::Duration::from_millis(10)).await;
            // client.close().await.unwrap();
        }
        // wait for finished, otherwise runtime exit before all tasks are done
        time::sleep(time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_multi_channel_open_close() {
        let client = Connection::open("localhost:5672").await.unwrap();

        let mut handles = vec![];

        for _ in 0..10 {
            let _ch = client.open_channel().await.unwrap();
            handles.push(tokio::spawn(async move {
                time::sleep(time::Duration::from_secs(1)).await;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_multi_conn_open_close() {
        let mut handles = vec![];
        for i in 0..10 {
            let handle = tokio::spawn(async move {
                let client = Connection::open("localhost:5672").await.unwrap();
                time::sleep(time::Duration::from_millis((i % 3) * 50 + 100)).await;
                client.close().await.unwrap();
            });
            handles.push(handle);
        }
        for h in handles {
            h.await.unwrap();
        }
    }
}
