//! Implementation of AMQP_0-9-1's Channel class compatible with RabbitMQ.
//!
//! It provides [`APIs`] to manage an AMQP [`Channel`].
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
//! [`APIs`]: struct.Channel.html#implementations
//! [`Channel`]: struct.Channel.html
//! [`close`]: struct.Channel.html#method.close
//!
use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use amqp_serde::types::AmqpChannelId;
use tokio::sync::{mpsc, oneshot};

use super::callbacks::ChannelCallback;
use crate::{
    api::{error::Error, Result},
    connection::Connection,
    frame::{CloseChannel, CloseChannelOk, Deliver, Flow, FlowOk, Frame, MethodHeader, Return},
    net::{ConnManagementCommand, IncomingMessage, OutgoingMessage},
    BasicProperties,
};
#[cfg(feature = "tracing")]
use tracing::{debug, error, info};

/// Combined message received by a consumer
/// 
/// Although all the fields are `Option<T>` type, the library guarantee
/// when user gets a message from receiver half of a consumer, 
/// all the fields have value of `Some<T>`.
pub struct ConsumerMessage {
    pub deliver: Option<Deliver>,
    pub basic_properties: Option<BasicProperties>,
    pub content: Option<Vec<u8>>,
}

/// Message buffer for a `return + content` sequence from server.
pub(crate) struct ReturnMessage {
    ret: Option<Return>,
    basic_properties: Option<BasicProperties>,
}

/// Command to register consumer of asynchronous delivered contents.
pub(crate) struct RegisterContentConsumer {
    consumer_tag: String,
    consumer_tx: mpsc::UnboundedSender<ConsumerMessage>,
}

/// Command to deregister consumer of asynchronous delivered contents.
///
/// Consumer should be deregistered when it is cancelled or the channel is closed.
pub(crate) struct DeregisterContentConsumer {
    consumer_tag: String,
}

/// Command to register sender to forward server's response to `get` request.
///
/// Server will respond `get-ok` + `message propertities` + `content body` in sequence,
/// so the sender should be mpsc instead of oneshot.
pub(crate) struct RegisterGetContentResponder {
    tx: mpsc::UnboundedSender<IncomingMessage>,
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
    DeregisterContentConsumer(DeregisterContentConsumer),
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
pub struct Channel {
    shared: Arc<SharedChannelInner>,
    /// A master channel is the one created by user, when drop, it will request
    /// to close the channel.
    /// A cloned channel has master = `false`.
    master: bool,
    /// associated connection
    connection: Connection,
}

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
    dispatcher_mgmt_tx: mpsc::UnboundedSender<DispatcherManagementCommand>,
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
        connection: Connection,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::UnboundedSender<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            master: true,
            connection,
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
            .send(DispatcherManagementCommand::RegisterChannelCallback(cmd))?;
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
            .send(DispatcherManagementCommand::RegisterOneshotResponder(cmd))?;
        acker_rx.await?;
        Ok(responder_rx)
    }

    pub fn channel_id(&self) -> AmqpChannelId {
        self.shared.channel_id
    }
    pub fn connection_name(&self) -> &str {
        self.connection.connection_name()
    }
    pub fn is_connection_open(&self) -> bool {
        self.connection.is_open()
    }
    /// Returns `true` if channel is open.
    pub fn is_open(&self) -> bool {
        self.shared.is_open.load(Ordering::Relaxed)
    }
    pub(crate) fn set_is_open(&self, is_open: bool) {
        self.shared.is_open.store(is_open, Ordering::Relaxed);
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
    pub async fn close(mut self) -> Result<()> {
        // if connection handler has been closed, no need to close
        if !self.shared.conn_mgmt_tx.is_closed() {
            // check if channel is open
            if let Ok(true) = self.shared.is_open.compare_exchange(
                true,
                false,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                #[cfg(feature = "tracing")]
                info!("close channel {}", self);
                self.close_handshake().await?;
                // not necessary, but to skip atomic compare at `drop`
                self.master = false;
            }
        }
        Ok(())
    }

    async fn close_handshake(&self) -> Result<()> {
        let responder_rx = self.register_responder(CloseChannelOk::header()).await?;
        synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, CloseChannel::default().into_frame()),
            responder_rx,
            Frame::CloseChannelOk,
            Error::ChannelCloseError
        )?;
        // deregister channel resource from connection handler,
        // so that dispatcher will exit automatically.
        let cmd = ConnManagementCommand::DeregisterChannelResource(self.channel_id());
        self.shared.conn_mgmt_tx.send(cmd).await?;
        Ok(())
    }
}

impl Clone for Channel {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            connection: self.connection.clone(),
            master: false,
        }
    }
}

impl Drop for Channel {
    /// When drops, try to gracefully shutdown the channel if it is still open.
    /// It is not guaranteed to succeed in a clean way because the connection
    /// may already be closed.
    ///
    /// User is recommended to explictly call the [`close`] method.
    ///
    /// [`close`]: struct.Channel.html#method.close
    fn drop(&mut self) {
        // only master channel will spawn task to close channel
        if self.master {
            // check if channel is open
            if let Ok(true) = self.shared.is_open.compare_exchange(
                true,
                false,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                #[cfg(feature = "tracing")]
                debug!("drop channel {}", self);

                let channel = self.clone();
                tokio::spawn(async move {
                    #[cfg(feature = "tracing")]
                    info!("close channel {} at drop", channel);
                    if let Err(err) = channel.close_handshake().await {
                        // Compliance: A peer that detects a socket closure without having received a Channel.Close-Ok
                        // handshake method SHOULD log the error.
                        #[cfg(feature = "tracing")]
                        error!(
                            "'{}' occurred at closing channel {} after drop",
                            err, channel,
                        );
                    } else {
                        #[cfg(feature = "tracing")]
                        info!("channel {} is closed OK after drop", channel);
                    }
                });
            }
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}] of connection {}",
            self.channel_id(),
            if self.is_open() { "open" } else { "closed" },
            self.connection,
        )
    }
}
///////////////////////////////////////////////////////////////////////////////
impl SharedChannelInner {
    fn new(
        is_open: AtomicBool,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::UnboundedSender<DispatcherManagementCommand>,
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
