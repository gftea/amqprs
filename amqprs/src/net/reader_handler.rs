use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{
    broadcast,
    mpsc::{Receiver, Sender},
};
use tracing::{debug, error, info};

use crate::{
    api::{callbacks::ConnectionCallback, connection::Connection},
    frame::{
        Close, CloseChannel, CloseChannelOk, CloseOk, Frame, MethodHeader, CONN_DEFAULT_CHANNEL,
    },
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

    async fn handle_close(
        &mut self,
        channel_id: AmqpChannelId,
        _method_header: &'static MethodHeader,
        close: Close,
    ) -> Result<(), Error> {
        assert_eq!(CONN_DEFAULT_CHANNEL, channel_id, "must be from channel 0");

        self.amqp_connection.set_open_state(false);

        // FIXME: we first respond OK to tell server we have received the message
        self.outgoing_tx
            .send((CONN_DEFAULT_CHANNEL, CloseOk::default().into_frame()))
            .await?;

        if let Some(ref mut callback) = self.callback {
            callback.close(&self.amqp_connection, close).await.unwrap();
            // self.callback.replace(callback);
        }

        Ok(())
    }

    async fn handle_close_ok(
        &mut self,
        channel_id: AmqpChannelId,
        method_header: &'static MethodHeader,
        close_ok: CloseOk,
    ) -> Result<(), Error> {
        assert_eq!(CONN_DEFAULT_CHANNEL, channel_id, "must be from channel 0");

        self.amqp_connection.set_open_state(false);

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

    /// If OK, user can continue to handle frame
    /// If NOK, user should stop consuming frame
    /// TODO: implement as Iterator, then user do not need to care about the error
    async fn handle_frame(&mut self, channel_id: AmqpChannelId, frame: Frame) -> Result<(), Error> {

        // handle only connection level frame, 
        // channel level frames are forwarded to corresponding channel dispatcher
        match frame {
            // TODO: Handle heartbeat
            Frame::HeartBeat(_) => {
                debug!("heartbeat, to be handled...");
                Ok(())
            }

            // Method frames for synchronous response
            Frame::OpenChannelOk(method_header, open_channel_ok) => {
                let responder = self
                    .channel_manager
                    .remove_responder(&channel_id, method_header)
                    .ok_or_else(|| Error::ImplementationError(format!("no responder found for OpenChannelOk of channel {}", channel_id)))?;

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
                self.handle_close_ok(channel_id, method_header, close_ok)
                    .await
            }

            // Method frames of asynchronous request
            // Server request to close connection
            Frame::Close(method_header, close) => {
                self.handle_close(channel_id, method_header, close).await
            }
            // TODO:
            Frame::Blocked(_method_header, _) | Frame::Unblocked(_method_header, _) => {
                todo!("handle asynchronous request")
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
                            "No dispatcher registered yet for channel {}, discard frame: {}",
                            channel_id, frame
                        );
                        Ok(())
                    }
                }
            }
        }
    }

    pub async fn run_until_shutdown(mut self) {
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
                            cmd.acker.send(id).expect("Acknowledge to command RegisterChannelResource should succeed");
                        },
                        ConnManagementCommand::RegisterResponder(cmd) => {
                            self.channel_manager.insert_responder(&cmd.channel_id, cmd.method_header, cmd.responder);
                            cmd.acker.send(()).expect("Acknowledge to command RegisterResponder should succeed");
                        },
                        ConnManagementCommand::RegisterConnectionCallback(cmd) => {
                            self.callback.replace(cmd.callback);
                        },

                    }
                }

                res = self.stream.read_frame() => {
                    match res {
                        Ok((channel_id, frame)) => {
                            if let Err(err) = self.handle_frame(channel_id, frame).await {
                                error!("Failed to handle frame, cause: {} ", err);
                                break;
                            }
                            if !self.amqp_connection.get_open_state() {
                                info!("Client has requested to shutdown connection or shutdown requested by server!");
                                break;
                            }
                        },
                        Err(err) => {
                            error!("Failed to read frame, cause: {}", err);
                            break;
                        },
                    }

                }
                else => {
                    break;
                }
            }
        }

        // `self` will drop, so the `self.shutdown_notifier`
        // all tasks which have `subscribed` to `shutdown_notifier` will be notified
        info!("Shutdown ReaderHandler!");
    }
}
