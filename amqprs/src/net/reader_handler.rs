use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::{
    sync::{
        broadcast,
        mpsc::{Receiver, Sender},
    },
    time,
};
use tracing::{debug, error, info, trace};

use crate::{
    api::{callbacks::ConnectionCallback, connection::Connection},
    frame::{CloseOk, Frame, DEFAULT_CONN_CHANNEL},
};

use super::{
    channel_manager::ChannelManager, BufReader, ConnManagementCommand, Error, OutgoingMessage,
};

/////////////////////////////////////////////////////////////////////////////

pub(crate) struct ReaderHandler {
    stream: BufReader,

    /// AMQ connection
    amqp_connection: Connection,

    /// sender half to forward outgoing message to `WriterHandler`
    outgoing_tx: Sender<OutgoingMessage>,

    /// receiver half to receive management command from AMQ Connection/Channel
    conn_mgmt_rx: Receiver<ConnManagementCommand>,

    /// connection level callback
    callback: Option<Box<dyn ConnectionCallback + Send + 'static>>,

    channel_manager: ChannelManager,

    /// Notify WriterHandler to shutdown.
    /// If reader handler exit first, it will notify writer handler to shutdown.
    /// If writer handler exit first, socket connection will be shutdown because the writer half drop,
    /// so socket read will return, and reader handler can detect connection shutdown without separate signal.
    #[allow(dead_code /* notify shutdown just by dropping the instance */)]
    shutdown_notifier: broadcast::Sender<()>,
}

impl ReaderHandler {
    pub fn new(
        stream: BufReader,
        amqp_connection: Connection,
        outgoing_tx: Sender<OutgoingMessage>,
        conn_mgmt_rx: Receiver<ConnManagementCommand>,
        channel_max: ShortUint,
        shutdown_notifier: broadcast::Sender<()>,
    ) -> Self {
        Self {
            stream,
            amqp_connection,
            outgoing_tx,
            conn_mgmt_rx,
            callback: None,
            channel_manager: ChannelManager::new(channel_max),
            shutdown_notifier,
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
                debug!("heartbeat received ...");
                Ok(())
            }

            // Method frames for synchronous response
            Frame::OpenChannelOk(method_header, open_channel_ok) => {
                let responder = self
                    .channel_manager
                    .remove_responder(&channel_id, method_header)
                    .expect("responder should be registered before receiving response");

                responder
                    .send(open_channel_ok.into_frame())
                    .map_err(|err_frame| {
                        Error::InternalChannelError(format!(
                            "failed to forward {} to client",
                            err_frame
                        ))
                    })
            }
            Frame::CloseOk(method_header, close_ok) => {
                self.amqp_connection.set_is_open(false);

                let responder = self
                    .channel_manager
                    .remove_responder(&channel_id, method_header)
                    .ok_or_else(|| {
                        Error::InternalChannelError(format!(
                            "No responder to forward frame {:?} to channel {}",
                            close_ok, channel_id
                        ))
                    })?;
                responder
                    .send(close_ok.into_frame())
                    .map_err(|response| Error::InternalChannelError(response.to_string()))?;
                Ok(())
            }

            // Method frames of asynchronous request
            // Server request to close connection
            Frame::Close(_, close) => {
                if let Some(ref mut callback) = self.callback {
                    if let Err(err) = callback.close(&self.amqp_connection, close).await {
                        debug!("connection close callback error, cause: {}.", err);
                        return Err(Error::CloseCallbackError);
                    }
                }
                // respond to server if no callback registered or callback succeed
                self.amqp_connection.set_is_open(false);

                self.outgoing_tx
                    .send((DEFAULT_CONN_CHANNEL, CloseOk::default().into_frame()))
                    .await?;

                Ok(())
            }

            Frame::Blocked(_, blocked) => {
                if let Some(ref mut callback) = self.callback {
                    callback
                        .blocked(&self.amqp_connection, blocked.reason.into())
                        .await;
                }
                Ok(())
            }
            Frame::Unblocked(_, _unblocked) => {
                if let Some(ref mut callback) = self.callback {
                    callback.unblocked(&self.amqp_connection).await;
                }
                Ok(())
            }
            _ => {
                let dispatcher = self.channel_manager.get_dispatcher(&channel_id);
                match dispatcher {
                    Some(dispatcher) => {
                        dispatcher.send(frame).await?;
                        Ok(())
                    }
                    None => {
                        error!(
                            "No dispatcher registered  for channel {}, discard frame: {}",
                            channel_id, frame
                        );
                        Ok(())
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
                        None => break,
                        Some(v) => v,
                    };
                    match command {
                        ConnManagementCommand::RegisterChannelResource(cmd) => {
                            let id = self.channel_manager.insert_resource(cmd.channel_id, cmd.resource);
                            cmd.acker.send(id).expect("acknowledge to command RegisterChannelResource should succeed");
                        },
                        ConnManagementCommand::UnregisterChannelResource(channel_id) => {
                            self.channel_manager.remove_resource(&channel_id);
                            debug!("channel {} resource unregistered from connection.", channel_id);
                        },
                        ConnManagementCommand::RegisterResponder(cmd) => {
                            self.channel_manager.insert_responder(&cmd.channel_id, cmd.method_header, cmd.responder);
                            cmd.acker.send(()).expect("acknowledge to command RegisterResponder should succeed");
                        },
                        ConnManagementCommand::RegisterConnectionCallback(cmd) => {
                            self.callback.replace(cmd.callback);
                            debug!("connection callback registered.");
                        },

                    }
                }
                res = self.stream.read_frame() => {
                    // any frame can be considered as heartbeat
                    expiration = time::Instant::now() + time::Duration::from_secs(max_interval);
                    trace!("server heartbeat deadline is updated to {:?}", expiration);

                    match res {
                        Ok((channel_id, frame)) => {
                            if let Err(err) = self.handle_frame(channel_id, frame).await {
                                error!("failed to handle frame, cause: {}!", err);
                                break;
                            }
                            if !self.amqp_connection.is_open() {
                                info!("client/server has requested to shutdown connection {}.", self.amqp_connection.connection_name());
                                break;
                            }
                        },
                        Err(err) => {
                            error!("failed to read frame, cause: {}!", err);
                            break;
                        },
                    }

                }
                _ = time::sleep_until(expiration) => {
                    // heartbeat deadline is updated whenever any frame received
                    // in normal case, expiration is always in the future due to received frame or heartbeats.
                    if expiration <= time::Instant::now() {
                        expiration = time::Instant::now() + time::Duration::from_secs(max_interval);
                        error!("missing heartbeat from server for connection {}", self.amqp_connection.connection_name());
                    }

                }
                else => {
                    break;
                }
            }
        }

        // FIXME: should here do Close/CloseOk to gracefully shutdown connection.
        // Best effort, ignore returned error
        self.amqp_connection.close().await.ok();

        // `self` will drop, so the `self.shutdown_notifier`
        // all tasks which have `subscribed` to `shutdown_notifier` will be notified
    }
}
