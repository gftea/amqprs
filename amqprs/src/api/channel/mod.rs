//! Implementation of AMQP_0-9-1's Channel class compatible with RabbitMQ.
//!
//! It provides [APIs][`Channel`] to manage an AMQP `Channel`.
//!
//! User should hold the channel object until no longer needs it, and call the [`close`] method
//! to gracefully shutdown the channel.
//!
//! When channel object is dropped, it will try with best effort
//! to close the channel, but no guarantee to handle close errors.
//!
//! Almost all methods of [`Channel`] accepts arguments, this module also contains
//! all argument types for each method.
//!
//! # Example
//! See [`crate`] documentation for quick start.
//! See details in documentation of each method.
//!
//! [`Channel`]: struct.Channel.html
//!
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use amqp_serde::types::AmqpChannelId;
use tokio::sync::{mpsc, oneshot};

use super::callbacks::ChannelCallback;
use crate::{
    api::{error::Error, Result},
    frame::{CloseChannel, CloseChannelOk, Deliver, Flow, FlowOk, Frame, MethodHeader, Return},
    net::{ConnManagementCommand, IncomingMessage, OutgoingMessage},
    BasicProperties,
};
use tracing::{debug, error, info};

pub(crate) const CONSUMER_MESSAGE_BUFFER_SIZE: usize = 32;

/// Aggregated buffer for a `deliver + content` sequence.
pub(crate) struct ConsumerMessage {
    deliver: Option<Deliver>,
    basic_properties: Option<BasicProperties>,
    content: Option<Vec<u8>>,
}

/// Aggregated buffer for a `return + content` sequence from server.
pub(crate) struct ReturnMessage {
    ret: Option<Return>,
    basic_properties: Option<BasicProperties>,
}

/// Command to register consumer of asynchronous delivered contents.
pub(crate) struct RegisterContentConsumer {
    consumer_tag: String,
    consumer_tx: mpsc::Sender<ConsumerMessage>,
}

/// Command to unregister consumer of asynchronous delivered contents.
///
/// Consumer should be unregistered when it is cancelled or the channel is closed.
pub(crate) struct UnregisterContentConsumer {
    consumer_tag: String,
}

/// Command to register sender to forward server's response to `get` request.
///
/// Server will respond `get-ok` + `message propertities` + `content body` in sequence,
/// so the sender should be mpsc instead of oneshot.
pub(crate) struct RegisterGetContentResponder {
    tx: mpsc::Sender<IncomingMessage>,
}

/// Command to register oneshot sender for response from server.
pub(crate) struct RegisterOneshotResponder {
    pub method_header: &'static MethodHeader,
    /// oneshot sender to forward response message from server.
    pub responder: oneshot::Sender<IncomingMessage>,
    // oneshot sender to acknowledge registration is done.
    pub acker: oneshot::Sender<()>,
}

/// Command to register channel callbacks
pub(crate) struct RegisterChannelCallback {
    pub callback: Box<dyn ChannelCallback + Send + 'static>,
}

/// List of management commands for channel dispatcher.
pub(crate) enum DispatcherManagementCommand {
    RegisterContentConsumer(RegisterContentConsumer),
    UnregisterContentConsumer(UnregisterContentConsumer),
    RegisterGetContentResponder(RegisterGetContentResponder),
    RegisterOneshotResponder(RegisterOneshotResponder),
    RegisterChannelCallback(RegisterChannelCallback),
}

/// Type represents an AMQP Channel.
///
/// First, create a new AMQP channel by `Connection's` method [`Connection::open_channel`].
///
/// Second, register callbacks for the channel by [`Channel::register_callback`].
///
/// Then, the channel is ready to use.
///
/// [`Connection::open_channel`]: ../connection/struct.Connection.html#method.open_channel
/// [`Channel::register_callback`]: struct.Channel.html#method.register_callback
///
#[derive(Debug, Clone)]
pub struct Channel {
    shared: Arc<SharedChannelInner>,
}

#[derive(Debug)]
pub(crate) struct SharedChannelInner {
    /// open state
    is_open: AtomicBool,
    /// channel id
    channel_id: AmqpChannelId,
    /// tx half to send message to `WriteHandler` task
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    /// tx half to send managment command to `ReaderHandler` task
    conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
    /// tx half to send management command to `ChannelDispatcher` task
    dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
}

impl SharedChannelInner {
    pub(in crate::api) fn new(
        is_open: AtomicBool,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            is_open,
            channel_id,
            outgoing_tx,
            conn_mgmt_tx,
            dispatcher_mgmt_tx,
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    /// Returns a new `Channel` instance.
    ///
    /// This does not open the channel. It is used internally by [`Connection::open_channel`].
    ///
    /// [`Connection::open_channel`]: ../connection/struct.Connection.html#method.open_channel
    pub(in crate::api) fn new(
        is_open: AtomicBool,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            shared: Arc::new(SharedChannelInner::new(
                is_open,
                channel_id,
                outgoing_tx,
                conn_mgmt_tx,
                dispatcher_mgmt_tx,
            )),
        }
    }

    /// Register callbacks for asynchronous message for the channel.
    ///
    /// User should always register callbacks. See [`callbacks`] documentation.
    ///
    /// # Errors
    ///
    /// Returns error if fail to send registration command.
    /// If returns [`Err`], user can try again until registration succeed.
    ///
    /// [`callbacks`]: ../callbacks/index.html
    pub async fn register_callback<F>(&self, callback: F) -> Result<()>
    where
        F: ChannelCallback + Send + 'static,
    {
        let cmd = RegisterChannelCallback {
            callback: Box::new(callback),
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterChannelCallback(cmd))
            .await?;
        Ok(())
    }

    /// Register oneshot responder for single message.
    ///
    /// Used for synchronous request/response protocol.
    async fn register_responder(
        &self,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<IncomingMessage>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterOneshotResponder {
            method_header,
            responder,
            acker,
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterOneshotResponder(cmd))
            .await?;
        acker_rx.await?;
        Ok(responder_rx)
    }

    /// Returns `true` if the channel's connection is already closed.
    pub(crate) fn is_connection_handler_closed(&self) -> bool {
        self.shared.conn_mgmt_tx.is_closed()
    }

    pub fn channel_id(&self) -> AmqpChannelId {
        self.shared.channel_id
    }

    pub(crate) fn set_is_open(&self, is_open: bool) {
        self.shared.is_open.store(is_open, Ordering::Relaxed);
    }

    /// Returns `true` if channel is open.
    pub fn is_open(&self) -> bool {
        self.shared.is_open.load(Ordering::Relaxed)
    }

    /// Asks the server to pause or restart the flow of content data.
    ///
    /// Ask to start the flow if input `active` = `true`, otherwise to pause.
    /// Also see [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#channel.flow).
    ///
    /// Returns `true` means the server will start/continue the flow, otherwise it will not.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    pub async fn flow(&self, active: bool) -> Result<bool> {
        let responder_rx = self.register_responder(FlowOk::header()).await?;
        let flow_ok = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, Flow::new(active).into_frame()),
            responder_rx,
            Frame::FlowOk,
            Error::ChannelUseError
        )?;
        Ok(flow_ok.active)
    }

    /// Ask the server to close the channel.
    ///
    /// To gracefully shutdown the channel, recommended to `close` the
    /// channel explicitly instead of relying on `drop`.
    ///
    /// This method consume the channel, so even it may return error,
    /// channel will anyway be dropped.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    /// Fail to close the channel may result in `channel leak` in server.
    pub async fn close(self) -> Result<()> {
        if let Ok(true) =
            self.shared
                .is_open
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            info!("close channel: {}", self.channel_id());

            // if connection has been closed, no need to close channel
            if !self.is_connection_handler_closed() {
                self.close_internal().await?;
            }
        }
        Ok(())
    }
    async fn close_internal(&self) -> Result<()> {
        let responder_rx = self.register_responder(CloseChannelOk::header()).await?;

        synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, CloseChannel::default().into_frame()),
            responder_rx,
            Frame::CloseChannelOk,
            Error::ChannelCloseError
        )?;
        // let cmd = ConnManagementCommand::UnregisterChannelResource(self.channel_id());
        // self.shared.conn_mgmt_tx.send(cmd).await?;

        Ok(())
    }
}

impl Drop for Channel {
    /// When drops, try to gracefully shutdown the channel if it is still open.
    /// It is not guaranteed to succeed in a clean way.
    ///
    /// User is recommended to explicitly close channel. See [module][`self`] documentation.    
    fn drop(&mut self) {
        if let Ok(true) =
            self.shared
                .is_open
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            debug!(
                "drop channel {}, spawn a task to close it.",
                self.channel_id()
            );

            let channel = self.clone();
            tokio::spawn(async move {
                if channel.is_connection_handler_closed() {
                    return;
                }
                info!("close channel: {} at drop", channel.channel_id());
                if let Err(err) = channel.close_internal().await {
                    // check if handler has exited
                    if !channel.is_connection_handler_closed() {
                        error!(
                            "'{}' occurred at closing channel {} after drop.",
                            err,
                            channel.channel_id(),
                        );
                    }
                }
            });
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
mod dispatcher;
pub(crate) use dispatcher::*;

mod basic;
mod confim;
mod exchange;
mod queue;
mod tx;

// public APIs
pub use basic::*;
pub use confim::*;
pub use exchange::*;
pub use queue::*;
pub use tx::*;
