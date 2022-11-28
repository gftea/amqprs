//! Traits for asynchronous message callbacks of [`Connection`] and [`Channel`].
//!
//! In AMQP_0-9-1 protocol, some messages (`methods` in AMQP's term) can be initiated by server.
//! These messages are handled asynchronously. The callback traits provide user the definition of interfaces.
//! User should define its own callback types and implement the traits [`ConnectionCallback`] and [`ChannelCallback`], then register the callbacks by
//! [`Connection::register_callback`] and [`Channel::register_callback`] respectively.
//! 
//! # Examples
//! See [`DefaultConnectionCallback`] and [`DefaultChannelCallback`] 
//! The default callback implementations are only for demo and debugging purposes. 
//! User is expected to implement its own callbacks.
//! 
//! [`Connection`]: ../connection/struct.Connection.html
//! [`Connection::register_callback`]: ../connection/struct.Connection.html#method.register_callback
//! [`Channel`]: ../channel/struct.Channel.html
//! [`Channel::register_callback`]: ../channel/struct.Channel.html#method.register_callback

use std::str::from_utf8;

use super::{channel::Channel, connection::Connection};
use crate::api::Result;
use crate::frame::Cancel;
use crate::{
    frame::{Ack, Blocked, Close, CloseChannel, Flow, Nack, Return, Unblocked},
    BasicProperties,
};
use async_trait::async_trait;
use tracing::{error, info};

/////////////////////////////////////////////////////////////////////////////
/// See [module level][`self`] documentation.
#[async_trait]
pub trait ConnectionCallback {
    /// Callback to handle `close` connection request from server.
    /// 
    /// Returns [`Ok`] to respond to server that the request is received and handled properly.
    /// If returns [`Err`], no response to server, which means server won't know whether the request 
    /// has been received by client, and might consider the connection isn't shutdown gracefully.
    async fn close(&mut self, connection: &Connection, close: Close) -> Result<()>;

    /// Callback to handle connection `blocked` indication from server 
    async fn blocked(&mut self, connection: &Connection, blocked: Blocked);

    /// Callback to handle connection `unblocked` indication from server 
    async fn unblocked(&mut self, connection: &Connection, blocked: Unblocked);
}

pub struct DefaultConnectionCallback;

#[async_trait]
impl ConnectionCallback for DefaultConnectionCallback {
    async fn close(&mut self, _connection: &Connection, close: Close) -> Result<()> {
        error!("{}!", close);
        Ok(())
    }

    async fn blocked(&mut self, _connection: &Connection, blocked: Blocked) {
        info!("connection blocked by server, reason: {}.", blocked.reason());
    }
    async fn unblocked(&mut self, _connection: &Connection, _blocked: Unblocked) {
        info!("connection unblocked by server.");
    }
}

/////////////////////////////////////////////////////////////////////////////
/// See [module level][`self`] documentation.
#[async_trait]
pub trait ChannelCallback {
    /// Callback to handle `close` channel request from server.
    /// 
    /// Returns [`Ok`] to respond to server that the request is received and handled properly.
    /// If returns [`Err`], no response to server, which means server won't know whether the request 
    /// has been received by client, and might consider the channel isn't closed.
    async fn close(&mut self, channel: &Channel, close: CloseChannel) -> Result<()>;

    /// Callback to handle server's request to `cancel` the consumer of current channel.
    /// 
    /// Returns [`Ok`] to respond to server that request has been received and the consumer will be cancelled.
    /// If returns [`Err`], no response to server and no consumer will be cancelled.  
    async fn cancel(&mut self, channel: &Channel,  cancel: Cancel) -> Result<()>;

    /// Callback to handle server's `flow` request to pause or restart the flow of sending content data.
    /// 
    /// Returns [`true`] to indicate to server that client starts sending data.
    /// Returns [`false`] to indicate to server that client stops sending data.
    async fn flow(&mut self, channel: &Channel, flow: Flow) -> Result<bool>;

    /// Callback to handle `ack` indication from server.
    /// 
    /// Only occurs in `publish confirm` mode, sent by server to acknowledges one or more messages published.
    async fn publish_ack(&mut self, channel: &Channel, ack: Ack);

    /// Callback to handle `nack` indication from server.
    /// 
    /// Only occurs in `publish confirm` mode, sent by server to inform publisher of unhandled messages.
    async fn publish_nack(&mut self, channel: &Channel, nack: Nack);

    /// Callback to handle `return` indication with undeliverable message from server.
    /// 
    /// The input variable [ret][`Return`] contains the reason why the message is returned.
    /// The input variable [basic_properties][`BasicProperties`] contains the propertities of the returned message.
    /// The input variable [content][`Vec<u8>`] contains the content of the returned message.
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

pub struct DefaultChannelCallback;

#[async_trait]
impl ChannelCallback for DefaultChannelCallback {
    async fn close(&mut self, _channel: &Channel, close: CloseChannel) -> Result<()> {
        error!("{}!", close);
        Ok(())
    }
    async fn cancel(&mut self, _channel: &Channel, cancel: Cancel) -> Result<()> {
        info!("receive cancel for consumer: {}.", cancel.consumer_tag());
        Ok(())
    }    
    async fn flow(&mut self, channel: &Channel, flow: Flow) -> Result<bool> {
        info!("channel flow request from server, {}.", flow.active());
        Ok(true)
    }
    async fn publish_ack(&mut self, channel: &Channel, ack: Ack) {
        info!("channel publish ack from server, {}.", ack.delivery_tag());
    }
    async fn publish_nack(&mut self, channel: &Channel, nack: Nack) {
        info!("channel publish nack from server, {}.", nack.delivery_tag());
    }
    async fn publish_return(
        &mut self,
        channel: &Channel,
        ret: Return,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        info!(">>>>> Publish Return Start <<<<<.");
        info!("{}.", ret);
        info!("{}.", basic_properties,);
        info!("{}.", from_utf8(&content).unwrap());
        info!(">>>>> Publish Return End <<<<<.");
    }
}
