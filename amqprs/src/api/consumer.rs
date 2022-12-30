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
use super::channel::{BasicAckArguments, Channel};
use crate::frame::{BasicProperties, Deliver};

use async_trait::async_trait;
#[cfg(feature = "tracing")]
use tracing::info;

/// Trait defines the callback interfaces for consuming asynchronous content data from server.
///
/// Continously consume the content data until the consumer is cancelled or channel is closed.
#[async_trait]
pub trait AsyncConsumer {
    /// Consume a delivery from Server.
    ///
    /// Every delivery combines a [Deliver](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.deliver) frame,
    /// message propertities, and content body.
    ///
    /// # Inputs
    ///
    /// `channel`: consumer's channel reference, typically used for acknowledge the delivery.
    ///
    /// `deliver`: see [basic.deliver](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.deliver)
    /// or [delivery metadata](https://www.rabbitmq.com/consumers.html#message-properties)
    ///
    /// `basic_properties`: see [message properties](https://www.rabbitmq.com/consumers.html#message-properties).
    ///
    /// `content`: the content body
    ///
    /// # Non-blocking and blocking consumer
    ///
    /// This method is invoked in a async task context, so its implementation should NOT be CPU bound.
    ///
    /// To handle CPU bound task, user can spawn a blocking task using
    /// [tokio::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
    /// for CPU bound job, and use [tokio's mpsc channel](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html#communicating-between-sync-and-async-code)
    /// to cummunicate between sync and async code.
    ///
    /// Also check [bridging async and blocking code](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html#related-apis-and-patterns-for-bridging-asynchronous-and-blocking-code).
    ///     
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
    callback: Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>,
}

impl DefaultConsumer {
    /// Return a new consumer.
    ///
    /// See [Acknowledgement Modes](https://www.rabbitmq.com/consumers.html#acknowledgement-modes)
    ///
    /// no_ack = [`true`] means automatic ack and should NOT send ACK to server.
    ///
    /// no_ack = [`false`] means manual ack, and should send ACK message to server.
    pub fn new(no_ack: bool, callback: Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>) -> Self {
        Self { no_ack, callback }
    }
}

impl Default for DefaultConsumer {
    fn default() -> Self {
        Self { no_ack: true, callback: None }
    }
}

#[async_trait]
impl AsyncConsumer for DefaultConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {
        #[cfg(feature = "tracing")]
        info!("consume delivery {} on channel {}", deliver, channel);

        // call callback if any callback is set
        if let Some(callback) = &self.callback {
            callback(_content);
        }

        // ack explicitly if manual ack
        if !self.no_ack {
            #[cfg(feature = "tracing")]
            info!("ack to delivery {} on channel {}", deliver, channel);
            let args = BasicAckArguments::new(deliver.delivery_tag(), false);
            channel.basic_ack(args).await.unwrap();
        }
    }
}
