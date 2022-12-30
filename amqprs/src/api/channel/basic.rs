use amqp_serde::types::AmqpDeliveryTag;
use tokio::sync::mpsc;
#[cfg(feature = "tracing")]
use tracing::{debug, trace};

use crate::{
    api::{
        channel::{
            ConsumerMessage, DispatcherManagementCommand, RegisterContentConsumer,
            CONSUMER_MESSAGE_BUFFER_SIZE,
        },
        consumer::AsyncConsumer,
        error::Error,
        FieldTable, Result,
    },
    frame::{
        Ack, BasicProperties, Cancel, CancelOk, Consume, ConsumeOk, ContentBody, ContentHeader,
        ContentHeaderCommon, Frame, Get, GetOk, Nack, Publish, Qos, QosOk, Recover, RecoverOk,
        Reject,
    },
};

#[cfg(feature = "compilance_assert")]
use crate::api::compilance_asserts::{assert_exchange_name, assert_queue_name};

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
///     .no_ack(true)
///     .exclusive(true)
///     .finish();
/// ```
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.consume).
///
/// [`basic_consume`]: struct.Channel.html#method.basic_consume
#[derive(Debug, Clone, Default)]
pub struct BasicConsumeArguments {
    /// Queue Name. Default: "".
    pub queue: String,
    /// Default: "".
    pub consumer_tag: String,
    /// Default: `false`.
    pub no_local: bool,
    /// Default: `false`.
    pub no_ack: bool,
    /// Default: `false`.
    pub exclusive: bool,
    /// Default: `false`.
    pub no_wait: bool,
    /// Default: empty table.
    pub arguments: FieldTable,
}

impl BasicConsumeArguments {
    /// Create new arguments with defaults.
    pub fn new(queue: &str, consumer_tag: &str) -> Self {
        #[cfg(feature = "compilance_assert")]
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
    impl_chainable_setter! {
        /// Chainable setter method.
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
    pub async fn basic_qos(&mut self, args: BasicQosArguments) -> Result<()> {
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
    /// Returns consumer tag if succeed.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.  
    pub async fn basic_consume<F>(&self, consumer: F, args: BasicConsumeArguments) -> Result<String>
    where
        // TODO: this is blocking callback, spawn blocking task in connection manager
        // to provide async callback, and spawn async task  in connection manager
        F: AsyncConsumer + Send + 'static,
    {
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

        // before start consume, park the dispatcher first,
        // unpark the dispatcher after we have added consumer into callback queue
        // self.park_notify.notify_one();

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

        self.spawn_consumer(consumer_tag.clone(), consumer).await?;

        Ok(consumer_tag)
    }

    /// Spawn consumer task
    async fn spawn_consumer<F>(&self, consumer_tag: String, mut consumer: F) -> Result<()>
    where
        F: AsyncConsumer + Send + 'static,
    {
        let (consumer_tx, mut consumer_rx): (
            mpsc::Sender<ConsumerMessage>,
            mpsc::Receiver<ConsumerMessage>,
        ) = mpsc::channel(CONSUMER_MESSAGE_BUFFER_SIZE);

        let ctag = consumer_tag.clone();
        let channel = self.clone();
        // spawn consumer task
        tokio::spawn(async move {
            #[cfg(feature = "tracing")]
            trace!("starts task for consumer {} on channel {}", ctag, channel);

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
                        #[cfg(feature = "tracing")]
                        debug!("exit task of consumer {}", ctag);
                        break;
                    }
                }
            }
        });

        // register consumer to dispatcher
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterContentConsumer(
                RegisterContentConsumer {
                    consumer_tag,
                    consumer_tx,
                },
            ))
            .await?;
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
            .send(DispatcherManagementCommand::DeregisterContentConsumer(cmd))
            .await?;
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

        let (tx, mut rx) = mpsc::channel(3);
        let command = RegisterGetContentResponder { tx };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterGetContentResponder(
                command,
            ))
            .await?;

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
    /// Returns error if any failure in comunication with server.        
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

        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, publish.into_frame()))
            .await?;

        let content_header = ContentHeader::new(
            ContentHeaderCommon {
                class: 60, // basic class
                weight: 0,
                body_size: content.len() as u64,
            },
            basic_properties,
        );
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, content_header.into_frame()))
            .await?;

        let content = ContentBody::new(content);
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, content.into_frame()))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::time;
    use tracing::Level;

    use crate::{
        api::{
            channel::{QueueBindArguments, QueueDeclareArguments},
            connection::{Connection, OpenConnectionArguments},
            consumer::DefaultConsumer,
        },
        frame::BasicProperties, DELIVERY_MODE_TRANSIENT,
    };

    use super::{BasicConsumeArguments, BasicPublishArguments};

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_basic_consume_auto_ack() {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami")
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
                .no_ack(true)
                .finish();

            channel
                .basic_consume(DefaultConsumer::new(args.no_ack, None), args)
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
        {
            let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami")
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
                .basic_consume(DefaultConsumer::new(args.no_ack, None), args)
                .await
                .unwrap();
            time::sleep(time::Duration::from_secs(1)).await;
        }
        // channel and connection drop, wait for close task to fnish
        time::sleep(time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_basic_publish() {
        {
            let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami")
                .connection_name("test_basic_publish")
                .finish();

            let connection = Connection::open(&args).await.unwrap();

            let channel = connection.open_channel(None).await.unwrap();

            let args = BasicPublishArguments::new("amq.topic", "eiffel._.amqprs._.tester");

            let basic_properties = BasicProperties::default()
                .with_content_type("application/json;charset=utf-8")
                .with_delivery_mode(DELIVERY_MODE_TRANSIENT)
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
}
