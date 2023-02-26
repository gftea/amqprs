//! Callback interfaces of asynchronous message for [`Connection`] and [`Channel`].
//!
//! In AMQP_0-9-1 protocol, some messages (`methods` in AMQP's term) can be initiated by server.
//! These messages are handled asynchronously by client via callbacks.
//!
//! User should define its own callback types and implement the traits [`ConnectionCallback`]
//! and [`ChannelCallback`].
//!
//! After open a connection, immediately register the callbacks by [`Connection::register_callback`].
//! After open a channel,  immediately register the callbacks by [`Channel::register_callback`].
//!
//! # Examples
//! See [`DefaultConnectionCallback`] and [`DefaultChannelCallback`] for simple example.
//!
//! The default callback implementations are only for demo and debugging purposes.
//! User is expected to implement its own callbacks.
//!
//! [`Connection`]: ../connection/struct.Connection.html
//! [`Connection::register_callback`]: ../connection/struct.Connection.html#method.register_callback
//! [`Channel`]: ../channel/struct.Channel.html
//! [`Channel::register_callback`]: ../channel/struct.Channel.html#method.register_callback

use super::{channel::Channel, connection::Connection};
use crate::api::Result;
use crate::frame::Cancel;
use crate::{
    frame::{Ack, Close, CloseChannel, Nack, Return},
    BasicProperties,
};
use async_trait::async_trait;
#[cfg(feature = "traces")]
use tracing::{error, info, warn};

/////////////////////////////////////////////////////////////////////////////
/// Callback interfaces for asynchronous `Connection` class message.
///
/// See [module][`self`] documentation for general guidelines.
#[async_trait]
pub trait ConnectionCallback {
    /// Callback to handle `close` connection request from server.
    ///
    /// Returns [`Ok`] to reply server that the request is received and
    /// handled properly.
    ///
    /// # Errors
    ///
    /// If returns [`Err`], no reply to server, which means server won't know
    /// whether the request has been received by client, and may consider
    /// the connection isn't shutdown.
    async fn close(&mut self, connection: &Connection, close: Close) -> Result<()>;

    /// Callback to handle connection `blocked` indication from server
    async fn blocked(&mut self, connection: &Connection, reason: String);

    /// Callback to handle connection `unblocked` indication from server
    async fn unblocked(&mut self, connection: &Connection);
}

/// Default type that implements `ConnectionCallback`.
///
/// For demo and debugging purpose only.
pub struct DefaultConnectionCallback;

#[async_trait]
impl ConnectionCallback for DefaultConnectionCallback {
    async fn close(&mut self, connection: &Connection, close: Close) -> Result<()> {
        #[cfg(feature = "traces")]
        error!(
            "handle close request for connection {}, cause: {}",
            connection, close
        );
        Ok(())
    }

    async fn blocked(&mut self, connection: &Connection, reason: String) {
        #[cfg(feature = "traces")]
        info!(
            "handle blocked notification for connection {}, reason: {}",
            connection, reason
        );
    }

    async fn unblocked(&mut self, connection: &Connection) {
        #[cfg(feature = "traces")]
        info!(
            "handle unblocked notification for connection {}",
            connection
        );
    }
}

/////////////////////////////////////////////////////////////////////////////
///  Callback interfaces for asynchronous `Channel` class message.
///
/// See [module][`self`] documentation for general guidelines.
#[async_trait]
pub trait ChannelCallback {
    /// Callback to handle `close` channel request from server.
    ///
    /// Returns [`Ok`] to reply server that the request is received and
    /// handled properly.
    ///
    /// # Errors
    ///
    /// If returns [`Err`], no reply to server, which means server won't know
    /// whether the request has been received by client, and may consider
    /// the channel isn't closed.
    async fn close(&mut self, channel: &Channel, close: CloseChannel) -> Result<()>;

    /// Callback to handle server's request to `cancel` the consumer of current channel.
    ///
    /// Returns [`Ok`] to reply server that request has been received and
    /// the consumer will be cancelled.
    ///
    /// # Errors
    ///
    /// If returns [`Err`], no reply to server and no consumer will be cancelled.
    async fn cancel(&mut self, channel: &Channel, cancel: Cancel) -> Result<()>;

    /// Callback to handle server's `flow` request to pause or restart
    /// the flow of sending content data.
    ///
    /// if `active` = [`true`], request to start, otherwise to pause.
    ///
    /// Returns [`true`] to indicate to server that client starts sending data.
    /// Returns [`false`] to indicate to server that client stops sending data.
    async fn flow(&mut self, channel: &Channel, active: bool) -> Result<bool>;

    /// Callback to handle `ack` indication from server.
    ///
    /// Only occurs in `publish confirm` mode, sent by server to acknowledges
    /// one or more messages published.
    async fn publish_ack(&mut self, channel: &Channel, ack: Ack);

    /// Callback to handle `nack` indication from server.
    ///
    /// Only occurs in `publish confirm` mode, sent by server to inform publisher
    /// of unhandled messages.
    async fn publish_nack(&mut self, channel: &Channel, nack: Nack);

    /// Callback to handle `return` indication with undeliverable message from server.
    ///
    /// The [ret][`Return`] contains the reason why the message is returned.
    ///
    /// The [basic_properties][`BasicProperties`] contains the propertities
    /// of the returned message.
    ///
    /// The [content][`Vec<u8>`] contains the body of the returned message.
    ///
    /// [`Return`]: ../struct.Return.html
    /// [`BasicProperties`]: ../struct.BasicProperties.html
    async fn publish_return(
        &mut self,
        channel: &Channel,
        ret: Return,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    );
}

/// Default type that implements `ChannelCallback`.
///
/// For demo and debugging purpose only.
pub struct DefaultChannelCallback;

#[async_trait]
impl ChannelCallback for DefaultChannelCallback {
    async fn close(&mut self, channel: &Channel, close: CloseChannel) -> Result<()> {
        #[cfg(feature = "traces")]
        error!(
            "handle close request for channel {}, cause: {}",
            channel, close
        );
        Ok(())
    }
    async fn cancel(&mut self, channel: &Channel, cancel: Cancel) -> Result<()> {
        #[cfg(feature = "traces")]
        warn!(
            "handle cancel request for consumer {} on channel {}",
            cancel.consumer_tag(),
            channel
        );
        Ok(())
    }
    async fn flow(&mut self, channel: &Channel, active: bool) -> Result<bool> {
        #[cfg(feature = "traces")]
        info!(
            "handle flow request active={} for channel {}",
            active, channel
        );
        Ok(true)
    }
    async fn publish_ack(&mut self, channel: &Channel, ack: Ack) {
        #[cfg(feature = "traces")]
        info!(
            "handle publish ack delivery_tag={} on channel {}",
            ack.delivery_tag(),
            channel
        );
    }
    async fn publish_nack(&mut self, channel: &Channel, nack: Nack) {
        #[cfg(feature = "traces")]
        warn!(
            "handle publish nack delivery_tag={} on channel {}",
            nack.delivery_tag(),
            channel
        );
    }
    async fn publish_return(
        &mut self,
        channel: &Channel,
        ret: Return,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        #[cfg(feature = "traces")]
        warn!(
            "handle publish return {} on channel {}, content size: {}",
            ret,
            channel,
            content.len()
        );
    }
}
