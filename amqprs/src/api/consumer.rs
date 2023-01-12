//! Callback interfaces of asynchronous content data consumer.
//!
//! The consumer is required by [`Channel::basic_consume`] or [`Channel::basic_consume_blocking`].
//! User should create its own consumer by implementing the trait [`AsyncConsumer`] or [`BlockingConsumer`].
//!
//! # Examples
//!
//! See implementation of [`DefaultConsumer`] and [`DefaultBlockingConsumer`].
//!
//! [`Channel::basic_consume`]: ../channel/struct.Channel.html#method.basic_consume
//! [`Channel::basic_consume_blocking`]: ../channel/struct.Channel.html#method.basic_consume_blocking
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
    /// This method is invoked in a async task context, so its implementation should NOT be CPU bound, otherwise it will starving the async runtime.
    ///
    /// For CPU bound task (blocking consumer), possible solution below
    /// 1. User can spawn a blocking task using [tokio::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
    ///    for CPU bound job, and use [tokio's mpsc channel](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html#communicating-between-sync-and-async-code)
    ///    to cummunicate between sync and async code.
    ///    If too many blocking tasks, user can create a thread pool shared by all blocking tasks, and this method is only to forward message
    ///    to corresponding blocking task.
    ///    Also check [bridging async and blocking code](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html#related-apis-and-patterns-for-bridging-asynchronous-and-blocking-code).
    ///
    /// 2. Create blocking consumer by implementing trait [`BlockingConsumer`], and use [`Channel::basic_consume_blocking`]
    ///    to start consuming message in a blocking context.
    ///
    /// [`Channel::basic_consume_blocking`]: ../channel/struct.Channel.html#method.basic_consume_blocking

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
        _basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {
        #[cfg(feature = "tracing")]
        info!("consume delivery {} on channel {}", deliver, channel);

        // ack explicitly if manual ack
        if !self.no_ack {
            #[cfg(feature = "tracing")]
            info!("ack to delivery {} on channel {}", deliver, channel);
            let args = BasicAckArguments::new(deliver.delivery_tag(), false);
            channel.basic_ack(args).await.unwrap();
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
/// Similar as [`AsyncConsumer`] but run in a blocking context, aiming for CPU bound task.
pub trait BlockingConsumer {
    /// Except that a blocking consumer will be run in a separate Thread, otherwise see explanation
    /// in  [`AsyncConsumer::consume`].
    ///
    /// If there are too many blocking consumers, user is recommended to use a thread pool for all
    /// blocking tasks. See possible solution in [`non-blocking and blocking consumer`].
    ///
    /// [`non-blocking and blocking consumer`]: trait.AsyncConsumer.html#non-blocking-and-blocking-consumer
    fn consume(
        &mut self, // use `&mut self` to make trait object to be `Sync`
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    );
}

/// Default type implements the [`BlockingConsumer`].
///
/// It is used for demo and debugging purposes only.
pub struct DefaultBlockingConsumer {
    no_ack: bool,
}

impl DefaultBlockingConsumer {
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

impl BlockingConsumer for DefaultBlockingConsumer {
    fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {
        #[cfg(feature = "tracing")]
        info!("consume delivery {} on channel {}", deliver, channel);

        // ack explicitly if manual ack
        if !self.no_ack {
            #[cfg(feature = "tracing")]
            info!("ack to delivery {} on channel {}", deliver, channel);
            let args = BasicAckArguments::new(deliver.delivery_tag(), false);
            // should call blocking version of API because we are in blocing context
            channel.basic_ack_blocking(args).unwrap();
        }
    }
}
