use std::sync::Arc;

use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::{
    sync::{
        broadcast,
        mpsc::{Receiver, Sender}, Notify,
    },
    task::yield_now,
    time,
};
#[cfg(feature = "traces")]
use tracing::{debug, error, info, trace, warn};

use crate::{
    api::{callbacks::ConnectionCallback, connection::Connection},
    frame::{CloseOk, Frame, DEFAULT_CONN_CHANNEL},
};

use super::{
    channel_manager::ChannelManager, BufIoReader, ConnManagementCommand, Error, OutgoingMessage,
};

/////////////////////////////////////////////////////////////////////////////

pub(crate) struct ReaderHandler {
    stream: BufIoReader,

    /// AMQ connection
    amqp_connection: Connection,

    /// sender half to forward outgoing message to `WriterHandler`
    outgoing_tx: Sender<OutgoingMessage>,

    /// receiver half to receive management command from AMQ Connection/Channel
    conn_mgmt_rx: Receiver<ConnManagementCommand>,

    /// AMQP protocol layer callbacks
    callback: Option<Box<dyn ConnectionCallback + Send + 'static>>,

    channel_manager: ChannelManager,

    /// Notify WriterHandler to shutdown.
    /// If reader handler exit first, it will notify writer handler to shutdown.
    /// If writer handler exit first, socket connection will be shutdown because the writer half drop,
    /// so socket read will return, and reader handler can detect connection shutdown without separate signal.
    #[allow(dead_code /* notify shutdown just by dropping the instance */)]
    shutdown_notifier: broadcast::Sender<()>,

    io_failure_notify: Arc<Notify>
}

impl ReaderHandler {
    pub fn new(
        stream: BufIoReader,
        amqp_connection: Connection,
        outgoing_tx: Sender<OutgoingMessage>,
        conn_mgmt_rx: Receiver<ConnManagementCommand>,
        channel_max: ShortUint,
        shutdown_notifier: broadcast::Sender<()>,
        io_failure_notify: Arc<Notify>
    ) -> Self {
        Self {
            stream,
            amqp_connection,
            outgoing_tx,
            conn_mgmt_rx,
            callback: None,
            channel_manager: ChannelManager::new(channel_max),
            shutdown_notifier,
            io_failure_notify,
        }
    }

    /// If OK, user can continue to handle frame
    /// If NOK, user should stop consuming frame
    /// TODO: implement as Iterator, then user do not need to care about the error
    async fn handle_frame(&mut self, channel_id: AmqpChannelId, frame: Frame) -> Result<(), Error> {
        // handle only connection level frame,
        // channel level frames are forwarded to corresponding channel dispatcher
        match frame {
            // any received frame can be considered as heartbeat
            // nothing to handle with heartbeat frame.
            Frame::HeartBeat(_) => {
                #[cfg(feature = "traces")]
                debug!("received heartbeat on connection {}", self.amqp_connection);
                Ok(())
            }

            // Method frames for synchronous response
            Frame::OpenChannelOk(method_header, open_channel_ok) => {
                let responder = self
                    .channel_manager
                    .remove_responder(&channel_id, method_header)
                    .expect("responder must be registered");

                responder
                    .send(open_channel_ok.into_frame())
                    .map_err(|err_frame| {
                        Error::SyncChannel(format!(
                            "failed to forward {} to connection {}",
                            err_frame, self.amqp_connection
                        ))
                    })
            }
            Frame::CloseOk(method_header, close_ok) => {
                self.amqp_connection.set_is_open(false);

                match self
                    .channel_manager
                    .remove_responder(&channel_id, method_header)
                {
                    Some(responder) => responder
                        .send(close_ok.into_frame())
                        .map_err(|response| Error::SyncChannel(response.to_string()))?,
                    None => {
                        #[cfg(feature = "traces")]
                        warn!(
                            "CloseOk responder not found, probably connection {} has dropped",
                            self.amqp_connection
                        );
                    }
                }

                #[cfg(feature = "traces")]
                info!("close connection {} OK", self.amqp_connection);

                // Try to yield for last sent message to be scheduled.
                yield_now().await;
                Ok(())
            }

            // Method frames of asynchronous request
            // Server request to close connection
            Frame::Close(_, close) => {
                if let Some(ref mut callback) = self.callback {
                    if let Err(err) = callback.close(&self.amqp_connection, close).await {
                        #[cfg(feature = "traces")]
                        error!(
                            "close callback error on connection {}, cause: {}",
                            self.amqp_connection, err
                        );
                        return Err(Error::Callback);
                    }
                } else {
                    #[cfg(feature = "traces")]
                    error!(
                        "callback not registered on connection {}",
                        self.amqp_connection
                    );
                }
                // respond to server if no callback registered or callback succeed
                self.amqp_connection.set_is_open(false);
                self.outgoing_tx
                    .send((DEFAULT_CONN_CHANNEL, CloseOk::default().into_frame()))
                    .await?;
                #[cfg(feature = "traces")]
                info!(
                    "server requests to shutdown connection {}",
                    self.amqp_connection
                );

                // Try to yield for last sent message to be scheduled.
                yield_now().await;
                Ok(())
            }

            Frame::Blocked(_, blocked) => {
                if let Some(ref mut callback) = self.callback {
                    callback
                        .blocked(&self.amqp_connection, blocked.reason.into())
                        .await;
                } else {
                    #[cfg(feature = "traces")]
                    error!(
                        "callback not registered on connection {}",
                        self.amqp_connection
                    );
                }
                Ok(())
            }
            Frame::Unblocked(_, _unblocked) => {
                if let Some(ref mut callback) = self.callback {
                    callback.unblocked(&self.amqp_connection).await;
                } else {
                    #[cfg(feature = "traces")]
                    error!(
                        "callback not registered on connection {}",
                        self.amqp_connection
                    );
                }
                Ok(())
            }
            // dispatch other frames to channel dispatcher
            _ => {
                let dispatcher = self.channel_manager.get_dispatcher(&channel_id);
                match dispatcher {
                    Some(dispatcher) => {
                        dispatcher.send(frame)?;
                        Ok(())
                    }
                    None => {
                        unreachable!(
                            "dispatcher must be registered for channel {} of {}",
                            channel_id, self.amqp_connection,
                        );
                    }
                }
            }
        }
    }

    pub async fn run_until_shutdown(mut self, heartbeat: ShortUint) {
        // max interval to consider heartbeat is timeout
        let max_interval: u64 = heartbeat.into();
        let mut expiration = time::Instant::now() + time::Duration::from_secs(max_interval);
        loop {
            tokio::select! {
                biased;

                command = self.conn_mgmt_rx.recv() => {
                    let command = match command {
                        None => {
                            // should never happen because `ReadHandler` holds
                            // a `Connection` itself
                            unreachable!("connection command channel is closed, {}", self.amqp_connection)
                        },
                        Some(v) => v,
                    };
                    match command {
                        ConnManagementCommand::RegisterChannelResource(cmd) => {
                            let id = self.channel_manager.insert_resource(cmd.channel_id, cmd.resource);
                            cmd.acker.send(id).expect("ack to command RegisterChannelResource must succeed");
                            #[cfg(feature="traces")]
                            debug!("register channel resource on connection {}", self.amqp_connection);

                        },
                        ConnManagementCommand::DeregisterChannelResource(channel_id) => {
                            self.channel_manager.remove_resource(&channel_id);
                            #[cfg(feature="traces")]
                            debug!("deregister channel {} from connection {}", channel_id, self.amqp_connection);
                        },
                        ConnManagementCommand::RegisterResponder(cmd) => {
                            self.channel_manager.insert_responder(&cmd.channel_id, cmd.method_header, cmd.responder);
                            cmd.acker.send(()).expect("ack to command RegisterResponder must succeed");
                        },
                        ConnManagementCommand::RegisterConnectionCallback(cmd) => {
                            self.callback.replace(cmd.callback);
                            #[cfg(feature="traces")]
                            debug!("callback registered on connection {}", self.amqp_connection);
                        },
                    }
                }
                res = self.stream.read_frame() => {
                    // any frame can be considered as heartbeat
                    expiration = time::Instant::now() + time::Duration::from_secs(max_interval);
                    #[cfg(feature="traces")]
                    trace!("server heartbeat deadline is updated to {:?}", expiration);

                    match res {
                        Ok((channel_id, frame)) => {
                            if let Err(err) = self.handle_frame(channel_id, frame).await {
                                self.io_failure_notify.notify_one();
                                #[cfg(feature="traces")]
                                error!("socket will be closed due to error of handling frame, cause: {}", err);
                                break;
                            }
                            if !self.amqp_connection.is_open() {
                                #[cfg(feature="traces")]
                                info!("connection {} is closed, shutting down socket I/O handlers", self.amqp_connection);
                                break;
                            }
                        },
                        Err(err) => {
                            self.io_failure_notify.notify_one();
                            #[cfg(feature="traces")]
                            error!("socket will be closed due to failure of reading frame, cause: {}", err);
                            break;
                        },
                    }
                }
                _ = time::sleep_until(expiration) => {
                    // heartbeat deadline is updated whenever any frame received
                    // in normal case, expiration is always in the future due to received frame or heartbeats.
                    if expiration <= time::Instant::now() {
                        expiration = time::Instant::now() + time::Duration::from_secs(max_interval);

                        // TODO: what to do with missing heartbeat?
                        // should call self.io_failure_notify.notify_one();?
                        #[cfg(feature="traces")]
                        error!("missing heartbeat from server for {}", self.amqp_connection);
                    }
                }
                else => {
                    break;
                }
            }
        }
        self.amqp_connection.set_is_open(false);

        // `self` will drop, so the `self.shutdown_notifier`
        // all tasks which have `subscribed` to `shutdown_notifier` will be notified
    }
}
