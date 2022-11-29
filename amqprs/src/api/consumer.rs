//! Callback interfaces of asynchronous content data consumer.
//! 
//! The consumer is required by [`Channel::basic_consume`].
//! User should create its own consumer by implementing the trait [`AsyncConsumer`].
//! 
//! # Examples
//! 
//! See implementation of [`DefaultConsumer`].
//! 
//! [`Channel::consume`]: ../channel/struct.Channel.html#method.basic_consume
//! 
use std::str::from_utf8;
use crate::frame::{BasicProperties, Deliver};
use super::channel::{BasicAckArguments, Channel};

use async_trait::async_trait;
use tracing::info;

/// Trait defines the callback interfaces for consuming asynchronous content data from server.
/// 
/// Continously consume the content data until the consumer is cancelled or channel is closed.
#[async_trait]
pub trait AsyncConsumer {
    /// Consume a delivered message and associated content data from Server.
    /// 
    /// Method is called in a async task context. 
    /// Implementation of this method should not be CPU bound (non-blocking). 
    /// 
    /// To handle CPU bound consumer, user can spawn a blocking task dedicated 
    /// for CPU bound job, and within this method, use channel to communicate
    /// with the blocking task.
    async fn consume(
        &mut self, // use `&mut self` to make trait object to be `Sync`
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    );
}

/// Default type implements the [`AsyncConsumer`].
/// 
/// It is used for demo and debugging purposes only.
pub struct DefaultConsumer {
    no_ack: bool,
}

impl DefaultConsumer {
    /// Return a new consumer.
    /// 
    /// See [Acknowledgement Modes](https://www.rabbitmq.com/consumers.html#acknowledgement-modes)
    /// 
    /// no_ack = [`true`] means automatic ack and should NOT send ACK to server.
    /// 
    /// no_ack = [`false`] means manual ack, and should send ACK message to server.
    pub fn new(no_ack: bool) -> Self {
        Self { no_ack }
    }
}

#[async_trait]
impl AsyncConsumer for DefaultConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        info!(">>>>> Consumer '{}' Start <<<<<.", deliver.consumer_tag());
        info!("{}.", deliver);
        info!("{}.", basic_properties,);
        info!("{}.", from_utf8(&content).unwrap());
        info!(">>>>> Consumer '{}' End <<<<<.", deliver.consumer_tag());

        // ack explicitly if manual ack
        if !self.no_ack {
            let mut args = BasicAckArguments::new();
            args.delivery_tag = deliver.delivery_tag();
            channel.basic_ack(args).await.unwrap();
        }
    }
}
