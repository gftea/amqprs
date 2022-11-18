use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, trace};

use crate::{
    frame::OpenChannelOk,
    net::{
        ChannelResource, ConnManagementCommand, IncomingMessage, OutgoingMessage, ReaderHandler,
        RegisterChannelResource, RegisterConnectionCallback, SplitConnection, WriterHandler,
    },
};
use crate::{
    frame::{
        Close, CloseOk, Frame, MethodHeader, Open, OpenChannel, ProtocolHeader, StartOk, TuneOk,
        CONN_DEFAULT_CHANNEL,
    },
    net::RegisterResponder,
};

use super::{
    callbacks::ConnectionCallback,
    channel::{Channel, ChannelDispatcher},
};
use super::{error::Error};
type Result<T> = std::result::Result<T, Error>;

/////////////////////////////////////////////////////////////////////////////

/////////////////////////////////////////////////////////////////////////////
pub struct ClientCapabilities {}

#[derive(Debug, Clone)]
pub struct ServerCapabilities {}

#[derive(Debug, Clone)]
pub struct Connection {
    shared: Arc<SharedConnectionInner>,
}

#[derive(Debug)]
struct SharedConnectionInner {
    capabilities: Option<ServerCapabilities>,
    // FIXME: what is identity of a connection?
    // conn_id: String
    channel_max: ShortUint,
    is_open: AtomicBool,
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
}

//  TODO: move below constants gto be part of static configuration of connection
const DISPATCHER_MESSAGE_BUFFER_SIZE: usize = 256;
const DISPATCHER_COMMAND_BUFFER_SIZE: usize = 64;

const OUTGOING_MESSAGE_BUFFER_SIZE: usize = 256;
const CONN_MANAGEMENT_COMMAND_BUFFER_SIZE: usize = 64;
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
        let  tune_ok = TuneOk::new(tune.channel_max(),tune.frame_max(), tune.heartbeat() );
        
        let channel_max = tune.channel_max();
        let _heartbeat = tune.heartbeat();
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
        let (outgoing_tx, outgoing_rx) = mpsc::channel(OUTGOING_MESSAGE_BUFFER_SIZE);
        let (conn_mgmt_tx, conn_mgmt_rx) = mpsc::channel(CONN_MANAGEMENT_COMMAND_BUFFER_SIZE);

        let shared = Arc::new(SharedConnectionInner {
            capabilities: None,
            channel_max,
            is_open: AtomicBool::new(true),
            outgoing_tx,
            conn_mgmt_tx,
        });

        let new_amq_conn = Self { shared };

        // spawn handlers for reader and writer of network connection
        new_amq_conn
            .spawn_handlers(connection, outgoing_rx, conn_mgmt_rx)
            .await;

        // register channel resource for connection's default channel
        new_amq_conn
            .register_channel_resource(Some(CONN_DEFAULT_CHANNEL), ChannelResource::new(None))
            .await
            .ok_or_else(|| {
                Error::ConnectionOpenError("Failed to register channel resource".to_string())
            })?;

        Ok(new_amq_conn)
    }

    async fn register_responder(
        &self,
        channel_id: AmqpChannelId,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<IncomingMessage>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterResponder {
            channel_id,
            method_header,
            responder,
            acker,
        };
        self.shared
            .conn_mgmt_tx
            .send(ConnManagementCommand::RegisterResponder(cmd))
            .await?;
        acker_rx.await?;
        Ok(responder_rx)
    }

    pub async fn register_callback<F>(&self, callback: F) -> Result<()>
    where
        F: ConnectionCallback + Send + 'static,
    {
        let cmd = RegisterConnectionCallback {
            callback: Box::new(callback),
        };
        self.shared
            .conn_mgmt_tx
            .send(ConnManagementCommand::RegisterConnectionCallback(cmd))
            .await?;
        Ok(())
    }

    pub(crate) fn set_open_state(&self, is_open: bool) {
        self.shared.is_open.store(is_open, Ordering::Relaxed);
    }
    pub fn get_open_state(&self) -> bool {
        self.shared.is_open.load(Ordering::Relaxed)
    }

    pub(crate) async fn register_channel_resource(
        &self,
        channel_id: Option<AmqpChannelId>,
        resource: ChannelResource,
    ) -> Option<AmqpChannelId> {
        let (acker, acker_rx) = oneshot::channel();
        let cmd = ConnManagementCommand::RegisterChannelResource(RegisterChannelResource {
            channel_id,
            resource,
            acker,
        });

        // If no channel id is given, it will be allocated by management task and included in acker response
        // otherwise same id will be received in response
        if let Err(err) = self.shared.conn_mgmt_tx.send(cmd).await {
            debug!("Failed to register channel resource, cause: {}", err);
            return None;
        }

        // expect a channel id in response
        match acker_rx.await {
            Ok(res) => {
                if let None = res {
                    debug!("Failed to register channel resource, error in channel id allocation");
                }
                res
            }
            Err(err) => {
                debug!("Failed to register channel resource, cause: {}", err);
                None
            }
        }
    }

    /// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
    pub(crate) async fn spawn_handlers(
        &self,
        connection: SplitConnection,
        outgoing_rx: mpsc::Receiver<OutgoingMessage>,
        conn_mgmt_rx: mpsc::Receiver<ConnManagementCommand>,
    ) {
        // Spawn two tasks for the connection
        // - one task for writer
        // - one task for reader

        let (shutdown_notifer, shutdown_listener) = broadcast::channel::<()>(1);

        let (reader, writer) = connection.into_split();

        // spawn task for read connection handler
        let rh = ReaderHandler::new(
            reader,
            self.clone(),
            self.shared.outgoing_tx.clone(),
            conn_mgmt_rx,
            self.shared.channel_max,
            shutdown_notifer,
        );
        tokio::spawn(async move {
            rh.run_until_shutdown().await;
        });

        // spawn task for write connection handler
        let wh = WriterHandler::new(writer, outgoing_rx, shutdown_listener);
        tokio::spawn(async move {
            wh.run_until_shutdown().await;
        });
    }

    /// open a AMQ channel
    pub async fn open_channel(&self) -> Result<Channel> {
        let (dispatcher_tx, dispatcher_rx) = mpsc::channel(DISPATCHER_MESSAGE_BUFFER_SIZE);
        let (dispatcher_mgmt_tx, dispatcher_mgmt_rx) =
            mpsc::channel(DISPATCHER_COMMAND_BUFFER_SIZE);

        // acquire the channel id to be used to open channel
        let channel_id = self
            .register_channel_resource(None, ChannelResource::new(Some(dispatcher_tx)))
            .await
            .ok_or_else(|| {
                Error::ChannelOpenError("Failed to register channel resource".to_string())
            })?;

        // register responder, use the acquired channel id
        let responder_rx = self
            .register_responder(channel_id, OpenChannelOk::header())
            .await?;

        synchronous_request!(
            self.shared.outgoing_tx,
            (channel_id, OpenChannel::default().into_frame()),
            responder_rx,
            Frame::OpenChannelOk,
            Error::ChannelOpenError
        )?;

        // create channel instance
        let channel = Channel::new(
            AtomicBool::new(true),
            channel_id,
            self.shared.outgoing_tx.clone(),
            self.shared.conn_mgmt_tx.clone(),
            dispatcher_mgmt_tx,
        );

        let dispatcher = ChannelDispatcher::new(channel.clone(), dispatcher_rx, dispatcher_mgmt_rx);
        dispatcher.spawn().await;

        Ok(channel)
    }

    /// close and consume the AMQ connection
    pub async fn close(self) -> Result<()> {
        self.set_open_state(false);

        // connection's close method , should use default channel id
        let responder_rx = self
            .register_responder(CONN_DEFAULT_CHANNEL, CloseOk::header())
            .await?;

        let close = Close::default();
        synchronous_request!(
            self.shared.outgoing_tx,
            (CONN_DEFAULT_CHANNEL, close.into_frame()),
            responder_rx,
            Frame::CloseOk,
            Error::ConnectionCloseError
        )?;

        Ok(())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Ok(true) =
            self.shared
                .is_open
                .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
        {
            trace!("drop and close connection");
            let conn = self.clone();
            tokio::spawn(async move {
                if let Err(err) = conn.close().await {
                    error!(
                        "error occurred during close connection when drop, cause: {}",
                        err
                    );
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;
    use tokio::time;
    use tracing::{Level, subscriber::SetGlobalDefaultError};

    #[tokio::test]
    async fn test_channel_open_close() {
        setup_logging(Level::DEBUG);
        {

            // test close on drop
            let connection = Connection::open("localhost:5672").await.unwrap();

            {
                // test close on drop
                let _channel = connection.open_channel().await.unwrap();
            }
            time::sleep(time::Duration::from_millis(10)).await;
        }
        // wait for finished, otherwise runtime exit before all tasks are done
        time::sleep(time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_multi_conn_open_close() {
        setup_logging(Level::DEBUG);

        let mut handles = vec![];
        for _ in 0..10 {
            let handle = tokio::spawn(async move {
                time::sleep(time::Duration::from_millis(200)).await;
                let connection = Connection::open("localhost:5672").await.unwrap();
                time::sleep(time::Duration::from_millis(200)).await;
                connection.close().await.unwrap();
            });
            handles.push(handle);
        }
        for h in handles {
            h.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_multi_channel_open_close() {
        setup_logging(Level::DEBUG);
        {
            let connection = Connection::open("localhost:5672").await.unwrap();
            let mut handles = vec![];

            for _ in 0..10 {
                let ch = connection.open_channel().await.unwrap();
                let handle = tokio::spawn(async move {
                    let _ch = ch;
                    time::sleep(time::Duration::from_millis(100)).await;
                });
                handles.push(handle);
            }
            for h in handles {
                h.await.unwrap();
            }
            time::sleep(time::Duration::from_millis(100)).await;
        }
        time::sleep(time::Duration::from_millis(100)).await;
    }

    pub fn setup_logging(level: Level) -> Result<(), SetGlobalDefaultError> {
        // construct a subscriber that prints formatted traces to stdout
        let subscriber = tracing_subscriber::fmt().with_max_level(level).finish();
    
        // use that subscriber to process traces emitted after this point
        tracing::subscriber::set_global_default(subscriber)
    }
    
}
