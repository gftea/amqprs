use std::{thread, time};

use amqp_serde::types::AmqpChannelId;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot,
};

use crate::net::{
    self, IncomingMessage, ManagementCommand, OutgoingMessage, RegisterResponder, SplitConnection,
};
use crate::{
    frame::{Close, Frame, Open, OpenChannel, ProtocolHeader, StartOk, TuneOk, CONN_CTRL_CHANNEL},
    net::InternalChannels,
};

use super::channel::{self, Channel};
use super::error::Error;
type Result<T> = std::result::Result<T, Error>;

pub struct ClientCapabilities {}
pub struct ServerCapabilities {}
pub struct Connection {
    capabilities: Option<ServerCapabilities>,
    is_open: bool,
    channel_id: AmqpChannelId,
    outgoing_tx: Sender<OutgoingMessage>,
    incoming_rx: Receiver<IncomingMessage>,
    mgmt_tx: Sender<ManagementCommand>,
}

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
        connection.write_frame(CONN_CTRL_CHANNEL, start_ok).await?;

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
            .write_frame(CONN_CTRL_CHANNEL, tune_ok.into_frame())
            .await?;

        // C: Open
        let open = Open::default().into_frame();
        connection.write_frame(CONN_CTRL_CHANNEL, open).await?;

        // S: OpenOk
        let (_, frame) = connection.read_frame().await?;
        get_expected_method!(
            frame,
            Frame::OpenOk,
            Error::ConnectionOpenError("open".to_string())
        )?;

        // spawn network management tasks and get internal channel' sender half.
        let InternalChannels {
            outgoing_tx,
            mgmt_tx,
        } = net::spawn(connection, channel_max).await;

        let (channel_id, incoming_rx) =
            Self::allocate_resource(&mgmt_tx, Some(CONN_CTRL_CHANNEL)).await?;

        Ok(Self {
            capabilities: None,
            is_open: true,
            channel_id,
            outgoing_tx,
            incoming_rx,
            mgmt_tx,
        })
    }
    async fn allocate_resource(
        mgmt_tx: &Sender<ManagementCommand>,
        channel_id: Option<AmqpChannelId>,
    ) -> Result<(AmqpChannelId, Receiver<IncomingMessage>)> {
        // allocate channel for receiving incoming message from server
        // register the sender half to handler, and keep the receiver half
        let (responder, incoming_rx) = mpsc::channel(1);
        let (acker, resp) = oneshot::channel();
        let cmd = ManagementCommand::RegisterResponder(RegisterResponder {
            channel_id,
            responder,
            acker,
        });

        // register responder for the channel.
        // If no channel id is given, it will be allocated by management task and included in acker response
        // otherwise same id will be received in response
        mgmt_tx
            .send(cmd)
            .await
            .map_err(|err| Error::ChannelAllocationError(err.to_string()))?;

        // expect a channel id in response
        match resp
            .await
            .map_err(|err| Error::ChannelAllocationError(err.to_string()))?
        {
            Some(channel_id) => Ok((channel_id, incoming_rx)),
            None => Err(Error::ChannelAllocationError(
                "Channel ID allocation failure".to_string(),
            )),
        }
    }
    /// close and consume the AMQ connection
    pub async fn close(mut self) -> Result<()> {
        synchronous_request!(
            self.outgoing_tx,
            (CONN_CTRL_CHANNEL, Close::default().into_frame()),
            self.incoming_rx,
            Frame::CloseOk,
            Error::ConnectionCloseError
        )?;
        self.is_open = false;
        Ok(())
    }

    /// open a AMQ channel
    pub async fn open_channel(&self) -> Result<Channel> {
        let (channel_id, incoming_rx) = Self::allocate_resource(&self.mgmt_tx, None).await?;

        let mut channel = Channel {
            is_open: false,
            channel_id,
            outgoing_tx: self.outgoing_tx.clone(),
            incoming_rx,
            mgmt_tx: self.mgmt_tx.clone(),
        };
        synchronous_request!(
            channel.outgoing_tx,
            (channel.channel_id, OpenChannel::default().into_frame()),
            channel.incoming_rx,
            Frame::OpenChannelOk,
            Error::ChannelOpenError
        )?;
        channel.is_open = true;
        Ok(channel)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if self.is_open {
            let tx = self.outgoing_tx.clone();
            let handle = tokio::spawn(async move {
                tx.send((CONN_CTRL_CHANNEL, Close::default().into_frame()))
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
                let channel = client.open_channel().await.unwrap();
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
        let mut client = Connection::open("localhost:5672").await.unwrap();

        let mut handles = vec![];

        for _ in 0..10 {
            let mut ch = client.open_channel().await.unwrap();
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
