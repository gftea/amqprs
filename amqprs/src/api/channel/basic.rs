use amqp_serde::types::AmqpDeliveryTag;
use tokio::sync::mpsc;
#[cfg(feature = "traces")]
use tracing::{debug, trace};

use crate::{
    api::{
        channel::{ConsumerMessage, DispatcherManagementCommand, RegisterContentConsumer},
        consumer::AsyncConsumer,
        error::Error,
        FieldTable, Result,
    },
    consumer::BlockingConsumer,
    frame::{
        Ack, BasicProperties, Cancel, CancelOk, Consume, ConsumeOk, ContentBody, ContentHeader,
        ContentHeaderCommon, Frame, Get, GetOk, Nack, Publish, Qos, QosOk, Recover, RecoverOk,
        Reject,
    },
};

#[cfg(feature = "compliance_assert")]
use crate::api::compliance_asserts::{assert_exchange_name, assert_queue_name};

use super::{Channel, DeregisterContentConsumer, RegisterGetContentResponder};
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_qos`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.qos).
///
/// [`basic_qos`]: struct.Channel.html#method.basic_qos
#[derive(Debug, Clone, Default)]
pub struct BasicQosArguments {
    /// Default: 0.
    pub prefetch_size: u32,
    /// Default: 0.
    pub prefetch_count: u16,
    /// Default: `false`.
    pub global: bool,
}

impl BasicQosArguments {
    /// Create new arguments with defaults.
    pub fn new(prefetch_size: u32, prefetch_count: u16, global: bool) -> Self {
        Self {
            prefetch_size,
            prefetch_count,
            global,
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        prefetch_size, u32
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        prefetch_count, u16
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        global, bool
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        self.clone()
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_consume`]
///
/// # Support chainable methods to build arguments
/// ```
/// # use amqprs::channel::BasicConsumeArguments;
///
/// let x = BasicConsumeArguments::new("q", "c")
///     .manual_ack(true)
///     .exclusive(false)
///     .finish();
/// ```
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.consume).
///
/// [`basic_consume`]: struct.Channel.html#method.basic_consume
#[derive(Debug, Clone, Default)]
pub struct BasicConsumeArguments {
    /// Target queue name. Must be provided.
    pub queue: String,
    /// Consumer identifier. Default: "" (server-generated).
    pub consumer_tag: String,
    /// Ignored by modern RabbitMQ releases. Default: `false`.
    pub no_local: bool,
    /// Should automatic acknowledgements be used? Default: `false`.
    pub no_ack: bool,
    /// Should this consumer be exclusive (the only one allowed on the target queue)? Default: `false`.
    pub exclusive: bool,
    /// Default: `false`.
    pub no_wait: bool,
    /// Default: empty table.
    pub arguments: FieldTable,
}

impl BasicConsumeArguments {
    /// Create new arguments with defaults.
    pub fn new(queue: &str, consumer_tag: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(queue);

        Self {
            queue: queue.to_owned(),
            consumer_tag: consumer_tag.to_owned(),
            no_local: false,
            no_ack: false,
            exclusive: false,
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        queue, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        consumer_tag, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_local, bool
    }
    impl_chainable_alias_setter! {
        /// Chainable setter method.
        auto_ack, no_ack, bool
    }
    pub fn manual_ack(&mut self, value: bool) -> &mut Self {
        self.no_ack = !value;
        self
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        #[deprecated(since="1.2.0", note="use the manual_ack builder method")]
        no_ack, bool
    }

    impl_chainable_setter! {
        /// Chainable setter method.
        exclusive, bool
    }

    impl_chainable_setter! {
        /// Chainable setter method.
        no_wait, bool
    }

    impl_chainable_setter! {
        /// Chainable setter method.
        arguments, FieldTable
    }

    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(&self.queue);

        self.clone()
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_cancel`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.cancel).
///
/// [`basic_cancel`]: struct.Channel.html#method.basic_cancel
#[derive(Debug, Clone, Default)]
pub struct BasicCancelArguments {
    /// Default: "".
    pub consumer_tag: String,
    /// Default: `false`.
    pub no_wait: bool,
}

impl BasicCancelArguments {
    /// Create new arguments with defaults.
    pub fn new(consumer_tag: &str) -> Self {
        Self {
            consumer_tag: consumer_tag.to_owned(),
            no_wait: false,
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        consumer_tag, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_wait, bool
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        self.clone()
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_get`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.get).
///
/// [`basic_get`]: struct.Channel.html#method.basic_get
#[derive(Debug, Clone, Default)]
pub struct BasicGetArguments {
    /// Queue name. Default: "".
    pub queue: String,
    /// Default: `false`.
    pub no_ack: bool,
}

impl BasicGetArguments {
    /// Create new arguments with defaults.
    pub fn new(queue: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(queue);

        Self {
            queue: queue.to_owned(),
            no_ack: false,
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        queue, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_ack, bool
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(&self.queue);

        self.clone()
    }
}

/// Tuple returned by [`Channel::basic_get`] method.
///
/// `get-ok` + `message propertities` + `message body`
///
/// [`Channel::basic_get`]: struct.Channel.html#method.basic_get
pub type GetMessage = (GetOk, BasicProperties, Vec<u8>);

////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_ack`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.ack).
///
/// [`basic_ack`]: struct.Channel.html#method.basic_ack
#[derive(Debug, Clone, Default)]
pub struct BasicAckArguments {
    /// Default: 0.
    pub delivery_tag: u64,
    /// Default: `false`.
    pub multiple: bool,
}

impl BasicAckArguments {
    /// Create new arguments with defaults.
    pub fn new(delivery_tag: AmqpDeliveryTag, multiple: bool) -> Self {
        Self {
            delivery_tag,
            multiple,
        }
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_nack`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.nack).
///
/// [`basic_nack`]: struct.Channel.html#method.basic_nack
#[derive(Debug, Clone)]
pub struct BasicNackArguments {
    /// Default: 0.
    pub delivery_tag: u64,
    /// Default: `false`'.
    pub multiple: bool,
    /// Default: `true`.
    pub requeue: bool,
}
impl Default for BasicNackArguments {
    fn default() -> Self {
        Self {
            delivery_tag: 0,
            multiple: false,
            requeue: true,
        }
    }
}
impl BasicNackArguments {
    /// Create new arguments with defaults.

    pub fn new(delivery_tag: u64, multiple: bool, requeue: bool) -> Self {
        Self {
            delivery_tag,
            multiple,
            requeue,
        }
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_reject`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.reject).
///
/// [`basic_reject`]: struct.Channel.html#method.basic_reject
#[derive(Debug, Clone)]
pub struct BasicRejectArguments {
    /// Default: 0.
    pub delivery_tag: u64,
    /// Default: `true`.
    pub requeue: bool,
}

impl Default for BasicRejectArguments {
    fn default() -> Self {
        Self {
            delivery_tag: 0,
            requeue: true,
        }
    }
}
impl BasicRejectArguments {
    /// Create new arguments with defaults.
    pub fn new(delivery_tag: AmqpDeliveryTag, requeue: bool) -> Self {
        Self {
            delivery_tag,
            requeue,
        }
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`basic_publish`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.publish).
///
/// [`basic_publish`]: struct.Channel.html#method.basic_publish
#[derive(Debug, Clone, Default)]
pub struct BasicPublishArguments {
    /// Exchange name. Default: "".
    pub exchange: String,
    /// Default: "".
    pub routing_key: String,
    /// Default: `false`.
    pub mandatory: bool,
    /// Default: `false`.
    pub immediate: bool,
}

impl BasicPublishArguments {
    /// Create new arguments with defaults.
    pub fn new(exchange: &str, routing_key: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_exchange_name(exchange);

        Self {
            exchange: exchange.to_owned(),
            routing_key: routing_key.to_owned(),
            mandatory: false,
            immediate: false,
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        exchange, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        routing_key, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        mandatory, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        immediate, bool
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_exchange_name(&self.exchange);

        self.clone()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// APIs for AMQP basic class.
impl Channel {
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.qos)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn basic_qos(&self, args: BasicQosArguments) -> Result<()> {
        let qos = Qos::new(args.prefetch_size, args.prefetch_count, args.global);
        let responder_rx = self.register_responder(QosOk::header()).await?;

        let _method = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, qos.into_frame()),
            responder_rx,
            Frame::QosOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.ack)
    ///
    /// Returns the consumer tag on success.
    ///
    /// # Errors
    ///
    /// Returns an error if a failure occurs while comunicating with the server.
    pub async fn basic_consume<F>(&self, consumer: F, args: BasicConsumeArguments) -> Result<String>
    where
        F: AsyncConsumer + Send + 'static,
    {
        let consumer_tag = self.request_basic_consume(args).await?;

        self.spawn_consumer(consumer_tag.clone(), consumer).await?;

        Ok(consumer_tag)
    }

    /// Similar as [`basic_consume`] but run the consumer in a blocking context.
    ///
    /// Returns the consumer tag on success.
    ///
    /// # Errors
    ///
    /// Returns an error if a failure occurs while comunicating with the server.
    ///
    /// [`basic_consume`]: struct.Channel.html#method.basic_consume
    pub async fn basic_consume_blocking<F>(
        &self,
        consumer: F,
        args: BasicConsumeArguments,
    ) -> Result<String>
    where
        F: BlockingConsumer + Send + 'static,
    {
        let consumer_tag = self.request_basic_consume(args).await?;

        self.spawn_blocking_consumer(consumer_tag.clone(), consumer)
            .await?;

        Ok(consumer_tag)
    }

    /// Similar to [`basic_consume`] but returns the raw unbounded [`UnboundedReceiver`]
    ///
    /// Returns the consumer tag and the [`UnboundedReceiver`] on success.
    ///
    /// If you were to stop consuming before the channel has been closed internally,
    /// you must call [`basic_cancel`] to make sure resources are cleaned up properly.
    ///
    /// if `no-ack` is false, [`basic_qos`] can be used to throttle the incoming flow from server.
    /// If `no-ack` is true, the `prefetch-size` and `prefetch-count` are ignored, flow control
    /// on application level maybe need to be introduced, othersie it relies on TCP backpresure.
    ///
    /// ```
    /// # use amqprs::{
    /// #     callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    /// #     channel::{BasicCancelArguments, BasicConsumeArguments, BasicPublishArguments, QueueDeclareArguments},
    /// #     connection::{Connection, OpenConnectionArguments},
    /// #     BasicProperties,
    /// # };
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let connection = Connection::open(&OpenConnectionArguments::new(
    /// #     "localhost",
    /// #     5672,
    /// #     "user",
    /// #     "bitnami",
    /// # ))
    /// # .await
    /// # .unwrap();
    /// #
    /// # connection
    /// #     .register_callback(DefaultConnectionCallback)
    /// #     .await
    /// #     .unwrap();
    /// #
    /// # let channel = connection.open_channel(None).await.unwrap();
    /// # channel
    /// #     .register_callback(DefaultChannelCallback)
    /// #     .await
    /// #     .unwrap();
    /// #
    /// #
    /// # let (queue_name, _, _) = channel
    /// #     .queue_declare(QueueDeclareArguments::default())
    /// #     .await
    /// #     .unwrap()
    /// #     .unwrap();
    /// #
    /// #
    /// # let content = String::from(
    /// #     r#"
    /// #         {
    /// #             "publisher": "example"
    /// #             "data": "Hello, amqprs!"
    /// #         }
    /// #     "#,
    /// # )
    /// # .into_bytes();
    /// #
    /// # // create arguments for basic_publish
    /// # let args = BasicPublishArguments::new("", &queue_name);
    /// #
    /// # channel
    /// #     .basic_publish(BasicProperties::default(), content, args)
    /// #     .await
    /// #     .unwrap();
    /// let args = BasicConsumeArguments::new(&queue_name, "basic_consumer")
    ///     .manual_ack(false)
    ///     .finish();
    ///
    /// let (ctag, mut messages_rx) = channel.basic_consume_rx(args).await.unwrap();
    ///
    /// // you will need to run this in `tokio::spawn` or `tokio::task::spawn_blocking`
    /// // if you want to do other things in parallel of message consumption.
    /// while let Some(msg) = messages_rx.recv().await {
    ///     // do smthing with msg
    /// #   break;
    /// }
    ///
    /// // Only needed when `messages_rx.recv().await` hasn't yet returned `None`
    /// if let Err(e) = channel.basic_cancel(BasicCancelArguments::new(&ctag)).await {
    ///     // handle err
    /// };
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if a failure occurs while comunicating with the server.
    ///
    /// [`basic_consume`]: struct.Channel.html#method.basic_consume
    /// [`basic_cancel`]: struct.Channel.html#method.basic_cancel
    /// [`basic_qos`]: struct.Channel.html#method.basic_qos
    /// [`UnboundedReceiver`]: https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.UnboundedReceiver.html
    pub async fn basic_consume_rx(
        &self,
        args: BasicConsumeArguments,
    ) -> Result<(String, mpsc::UnboundedReceiver<ConsumerMessage>)> {
        let consumer_tag = self.request_basic_consume(args).await?;

        let (consumer_tx, consumer_rx): (
            mpsc::UnboundedSender<ConsumerMessage>,
            mpsc::UnboundedReceiver<ConsumerMessage>,
        ) = mpsc::unbounded_channel();

        self.register_consumer(consumer_tag.clone(), consumer_tx)
            .await?;

        Ok((consumer_tag, consumer_rx))
    }

    /// Send basic consume request to server
    async fn request_basic_consume(&self, args: BasicConsumeArguments) -> Result<String> {
        let BasicConsumeArguments {
            queue,
            consumer_tag,
            no_local,
            no_ack,
            exclusive,
            no_wait,
            arguments,
        } = args;
        let mut consume = Consume::new(
            0,
            queue.try_into().unwrap(),
            consumer_tag.clone().try_into().unwrap(),
            arguments,
        );
        consume.set_no_local(no_local);
        consume.set_no_ack(no_ack);
        consume.set_exclusive(exclusive);
        consume.set_nowait(no_wait);
        let consumer_tag = if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, consume.into_frame()))
                .await?;
            consumer_tag
        } else {
            let responder_rx = self.register_responder(ConsumeOk::header()).await?;

            let method = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, consume.into_frame()),
                responder_rx,
                Frame::ConsumeOk,
                Error::ChannelUseError
            )?;
            method.consumer_tag.into()
        };
        Ok(consumer_tag)
    }

    /// Spawn async consumer task
    async fn spawn_consumer<F>(&self, consumer_tag: String, mut consumer: F) -> Result<()>
    where
        F: AsyncConsumer + Send + 'static,
    {
        let (consumer_tx, mut consumer_rx): (
            mpsc::UnboundedSender<ConsumerMessage>,
            mpsc::UnboundedReceiver<ConsumerMessage>,
        ) = mpsc::unbounded_channel();

        let ctag = consumer_tag.clone();
        let channel = self.clone_as_secondary();

        // spawn consumer task
        tokio::spawn(async move {
            #[cfg(feature = "traces")]
            trace!(
                "starts task for async consumer {} on channel {}",
                ctag,
                channel
            );

            loop {
                match consumer_rx.recv().await {
                    Some(mut msg) => {
                        consumer
                            .consume(
                                &channel,
                                msg.deliver.take().unwrap(),
                                msg.basic_properties.take().unwrap(),
                                msg.content.take().unwrap(),
                            )
                            .await;
                    }
                    None => {
                        #[cfg(feature = "traces")]
                        debug!("exit task of async consumer {}", ctag);
                        break;
                    }
                }
            }
        });

        self.register_consumer(consumer_tag, consumer_tx).await?;
        Ok(())
    }

    /// Spawn blocking consumer task
    async fn spawn_blocking_consumer<F>(&self, consumer_tag: String, mut consumer: F) -> Result<()>
    where
        F: BlockingConsumer + Send + 'static,
    {
        let (consumer_tx, mut consumer_rx): (
            mpsc::UnboundedSender<ConsumerMessage>,
            mpsc::UnboundedReceiver<ConsumerMessage>,
        ) = mpsc::unbounded_channel();

        let ctag = consumer_tag.clone();
        let channel = self.clone_as_secondary();

        // spawn blocking consumer task
        tokio::task::spawn_blocking(move || {
            #[cfg(feature = "traces")]
            trace!(
                "starts task for blocking consumer {} on channel {}",
                ctag,
                channel
            );

            loop {
                match consumer_rx.blocking_recv() {
                    Some(mut msg) => {
                        consumer.consume(
                            &channel,
                            msg.deliver.take().unwrap(),
                            msg.basic_properties.take().unwrap(),
                            msg.content.take().unwrap(),
                        );
                    }
                    None => {
                        #[cfg(feature = "traces")]
                        debug!("exit task of blocking consumer {}", ctag);
                        break;
                    }
                }
            }
        });

        self.register_consumer(consumer_tag, consumer_tx).await?;
        Ok(())
    }

    /// register consumer in dispatcher
    async fn register_consumer(
        &self,
        consumer_tag: String,
        consumer_tx: mpsc::UnboundedSender<ConsumerMessage>,
    ) -> Result<()> {
        self.shared.dispatcher_mgmt_tx.send(
            DispatcherManagementCommand::RegisterContentConsumer(RegisterContentConsumer {
                consumer_tag,
                consumer_tx,
            }),
        )?;
        Ok(())
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.ack)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn basic_ack(&self, args: BasicAckArguments) -> Result<()> {
        let ack = Ack::new(args.delivery_tag, args.multiple);
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, ack.into_frame()))
            .await?;
        Ok(())
    }

    /// Blocking version of [`basic_ack`], should be invoked in blocking context.
    ///
    /// # Panics
    ///
    /// Panic if invoked in async context.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    ///
    /// [`basic_ack`]: struct.Channel.html#method.basic_ack
    pub fn basic_ack_blocking(&self, args: BasicAckArguments) -> Result<()> {
        let ack = Ack::new(args.delivery_tag, args.multiple);
        self.shared
            .outgoing_tx
            .blocking_send((self.shared.channel_id, ack.into_frame()))?;
        Ok(())
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.nack)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn basic_nack(&self, args: BasicNackArguments) -> Result<()> {
        let mut nack = Nack::new(args.delivery_tag);
        nack.set_multiple(args.multiple);
        nack.set_requeue(args.requeue);
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, nack.into_frame()))
            .await?;
        Ok(())
    }

    /// Blocking version of [`basic_nack`], should be invoked in blocking context.
    ///
    /// # Panics
    ///
    /// Panic if invoked in async context.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    ///
    /// [`basic_nack`]: struct.Channel.html#method.basic_nack
    pub fn basic_nack_blocking(&self, args: BasicNackArguments) -> Result<()> {
        let mut nack = Nack::new(args.delivery_tag);
        nack.set_multiple(args.multiple);
        nack.set_requeue(args.requeue);
        self.shared
            .outgoing_tx
            .blocking_send((self.shared.channel_id, nack.into_frame()))?;
        Ok(())
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.reject)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn basic_reject(&self, args: BasicRejectArguments) -> Result<()> {
        let reject = Reject::new(args.delivery_tag, args.requeue);
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, reject.into_frame()))
            .await?;
        Ok(())
    }

    /// Blocking version of `basic_reject`
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub fn basic_reject_blocking(&self, args: BasicRejectArguments) -> Result<()> {
        let reject = Reject::new(args.delivery_tag, args.requeue);
        self.shared
            .outgoing_tx
            .blocking_send((self.shared.channel_id, reject.into_frame()))?;
        Ok(())
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.cancel)
    ///
    /// Returns consumer tag if succeed.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn basic_cancel(&self, args: BasicCancelArguments) -> Result<String> {
        let BasicCancelArguments {
            consumer_tag,
            no_wait,
        } = args;

        let cancel = Cancel::new(consumer_tag.clone().try_into().unwrap(), no_wait);

        let consumer_tag = if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, cancel.into_frame()))
                .await?;
            consumer_tag
        } else {
            let responder_rx = self.register_responder(CancelOk::header()).await?;

            let cancel_ok = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, cancel.into_frame()),
                responder_rx,
                Frame::CancelOk,
                Error::ChannelUseError
            )?;
            cancel_ok.consumer_tag.into()
        };

        let consumer_tag2 = consumer_tag.clone();
        let cmd = DeregisterContentConsumer { consumer_tag };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::DeregisterContentConsumer(cmd))?;
        Ok(consumer_tag2)
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.get)
    ///
    /// Either returns a tuple [`GetMessage`] or [`None`] if no message available.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn basic_get(&self, args: BasicGetArguments) -> Result<Option<GetMessage>> {
        let get = Get::new(0, args.queue.try_into().unwrap(), args.no_ack);

        let (tx, mut rx) = mpsc::unbounded_channel();
        let command = RegisterGetContentResponder { tx };
        self.shared.dispatcher_mgmt_tx.send(
            DispatcherManagementCommand::RegisterGetContentResponder(command),
        )?;

        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, get.into_frame()))
            .await?;
        let get_ok = match rx.recv().await.ok_or_else(|| {
            Error::InternalChannelError("failed to receive response to Get".to_string())
        })? {
            Frame::GetEmpty(_, _) => return Ok(None),
            Frame::GetOk(_, get_ok) => get_ok,
            _ => unreachable!("expect GetOk or GetEmpty"),
        };

        let basic_properties = match rx.recv().await.ok_or_else(|| {
            Error::InternalChannelError("failed to receive Get ContentHeader".to_string())
        })? {
            Frame::ContentHeader(header) => header.basic_properties,
            _ => unreachable!("expect ContentHeader"),
        };

        let content = match rx.recv().await.ok_or_else(|| {
            Error::InternalChannelError("failed to receive Get ContentBody".to_string())
        })? {
            Frame::ContentBody(content) => content.inner,
            _ => unreachable!("expect ContentBody"),
        };
        Ok(Some((get_ok, basic_properties, content)))
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.recover)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn basic_recover(&self, requeue: bool) -> Result<()> {
        let recover = Recover::new(requeue);

        let responder_rx = self.register_responder(RecoverOk::header()).await?;

        let _method = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, recover.into_frame()),
            responder_rx,
            Frame::RecoverOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.publish)
    ///
    /// # Errors
    ///
    /// Returns error in case of a network I/O failure. For data safety, use
    /// [publisher confirms](https://rabbitmq.com/publishers.html#data-safety).
    pub async fn basic_publish(
        &self,
        basic_properties: BasicProperties,
        content: Vec<u8>,
        args: BasicPublishArguments,
    ) -> Result<()> {
        let mut publish = Publish::new(
            0,
            args.exchange.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
        );
        publish.set_mandatory(args.mandatory);
        publish.set_immediate(args.immediate);

        let content_header = ContentHeader::new(
            ContentHeaderCommon {
                class: 60, // basic class
                weight: 0,
                body_size: content.len() as u64,
            },
            basic_properties,
        );

        let publish_combo =
            Frame::PublishCombo(publish, Box::new(content_header), ContentBody::new(content));
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, publish_combo))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::callbacks::{DefaultChannelCallback, DefaultConnectionCallback};
    use crate::test_utils::setup_logging;
    use crate::{
        api::{
            channel::{QueueBindArguments, QueueDeclareArguments},
            connection::{Connection, OpenConnectionArguments},
            consumer::DefaultConsumer,
        },
        frame::BasicProperties,
        DELIVERY_MODE_TRANSIENT,
    };
    use tokio::time;

    use super::{BasicConsumeArguments, BasicPublishArguments, BasicQosArguments};

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_basic_consume_auto_ack() {
        setup_logging();

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
            .connection_name("test_basic_consume_auto_ack")
            .finish();
        let connection = Connection::open(&args).await.unwrap();

        {
            let channel = connection.open_channel(None).await.unwrap();
            let (queue_name, ..) = channel
                .queue_declare(QueueDeclareArguments::default())
                .await
                .unwrap()
                .unwrap();
            channel
                .queue_bind(QueueBindArguments::new(
                    &queue_name,
                    "amq.topic",
                    "eiffel.#",
                ))
                .await
                .unwrap();

            let args = BasicConsumeArguments::new(&queue_name, "test_auto_ack")
                .auto_ack(true)
                .finish();

            channel
                .basic_consume(DefaultConsumer::new(args.no_ack), args)
                .await
                .unwrap();
            time::sleep(time::Duration::from_secs(1)).await;
        }
        // channel drops, wait for close handshake done.
        time::sleep(time::Duration::from_secs(1)).await;
        // connection drops
    }

    #[tokio::test]
    async fn test_basic_consume_manual_ack() {
        setup_logging();
        {
            let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
                .connection_name("test_basic_consume_manual_ack")
                .finish();

            let connection = Connection::open(&args).await.unwrap();

            let channel = connection.open_channel(None).await.unwrap();
            let (queue_name, ..) = channel
                .queue_declare(QueueDeclareArguments::default())
                .await
                .unwrap()
                .unwrap();
            channel
                .queue_bind(QueueBindArguments::new(
                    &queue_name,
                    "amq.topic",
                    "eiffel.#",
                ))
                .await
                .unwrap();

            let args = BasicConsumeArguments::new(&queue_name, "test_manual_ack");
            channel
                .basic_consume(DefaultConsumer::new(args.no_ack), args)
                .await
                .unwrap();
            time::sleep(time::Duration::from_secs(1)).await;
        }
        // channel and connection drop, wait for close task to fnish
        time::sleep(time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_basic_publish() {
        setup_logging();
        {
            let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
                .connection_name("test_basic_publish")
                .finish();

            let connection = Connection::open(&args).await.unwrap();

            let channel = connection.open_channel(None).await.unwrap();

            let args = BasicPublishArguments::new("amq.topic", "eiffel._.amqprs._.tester");

            let basic_properties = BasicProperties::default()
                .with_content_type("application/json;charset=utf-8")
                .with_persistence(true)
                .finish();

            let content = String::from(
            r#"
                {
                    "meta": {"id": "f9d42464-fceb-4282-be95-0cd98f4741b0", "type": "PublishTester", "version": "4.0.0", "time": 1640035100149},
                    "data": { "customData": []},
                    "links": [{"type": "BASE", "target": "fa321ff0-faa6-474e-aa1d-45edf8c99896"}]}
            "#
            ).into_bytes();

            channel
                .basic_publish(basic_properties, content, args)
                .await
                .unwrap();
            time::sleep(time::Duration::from_secs(1)).await;
        }
        // channel and connection drop, wait for close task to fnish
        time::sleep(time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_basic_qos() {
        setup_logging();

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");
        let connection = Connection::open(&args).await.unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();

        let (queue_name, ..) = channel
            .queue_declare(QueueDeclareArguments::default())
            .await
            .unwrap()
            .unwrap();
        channel
            .queue_bind(QueueBindArguments::new(
                &queue_name,
                "amq.topic",
                "eiffel.#",
            ))
            .await
            .unwrap();

        let args = BasicConsumeArguments::new(&queue_name, "test_basic_qos");
        channel
            .basic_consume(DefaultConsumer::new(args.no_ack), args)
            .await
            .unwrap();

        // set qos
        channel
            .basic_qos(BasicQosArguments::new(0, 100, false))
            .await
            .unwrap();

        // try recover
        channel.basic_recover(true).await.unwrap();

        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
