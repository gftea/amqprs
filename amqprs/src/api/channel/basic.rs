use tokio::sync::mpsc;
use tracing::{trace, debug};

use crate::{
    api::{
        channel::{
            ConsumerMessage, DispatcherManagementCommand, RegisterContentConsumer,
            CONSUMER_MESSAGE_BUFFER_SIZE,
        },
        consumer::AsyncConsumer,
        error::Error,
        Result
    },
    frame::{
        Ack, BasicProperties, Cancel, CancelOk, Consume, ConsumeOk, ContentBody, ContentHeader,
        ContentHeaderCommon, Frame, Get, GetOk, Nack, Publish, Qos, QosOk, Recover, RecoverOk,
        Reject,
    },
};

use super::{
    Channel, RegisterGetContentResponder, ServerSpecificArguments,
    UnregisterContentConsumer,
};

#[derive(Debug, Clone)]
pub struct BasicQosArguments {
    pub prefetch_size: u32,
    pub prefetch_count: u16,
    pub global: bool,
}

impl BasicQosArguments {
    pub fn new() -> Self {
        Self {
            prefetch_size: 0,
            prefetch_count: 0,
            global: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicConsumeArguments {
    pub queue: String,
    pub consumer_tag: String,
    pub no_local: bool,
    // In automatic acknowledgement mode,
    // a message is considered to be successfully delivered immediately after it is sent
    pub no_ack: bool,
    pub exclusive: bool,
    pub no_wait: bool,
    pub arguments: ServerSpecificArguments,
}

impl BasicConsumeArguments {
    pub fn new() -> Self {
        Self {
            queue: "".to_string(),
            consumer_tag: "".to_string(),
            no_local: false,
            no_ack: false,
            exclusive: false,
            no_wait: false,
            arguments: ServerSpecificArguments::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicCancelArguments {
    pub consumer_tag: String,
    pub no_wait: bool,
}

impl BasicCancelArguments {
    pub fn new(consumer_tag: String) -> Self {
        Self {
            consumer_tag,
            no_wait: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicGetArguments {
    pub queue: String,
    pub no_ack: bool,
}

impl BasicGetArguments {
    pub fn new() -> Self {
        Self {
            queue: "".to_string(),
            no_ack: false,
        }
    }
}

#[derive(Debug)]
pub struct GetMessage {
    pub get_ok: GetOk,
    pub basic_properties: BasicProperties,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BasicAckArguments {
    pub delivery_tag: u64,
    pub multiple: bool,
}

impl BasicAckArguments {
    pub fn new() -> Self {
        Self {
            delivery_tag: 0,
            multiple: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicNackArguments {
    pub delivery_tag: u64,
    pub multiple: bool,
    pub requeue: bool,
}

impl BasicNackArguments {
    pub fn new() -> Self {
        Self {
            delivery_tag: 0,
            multiple: false,
            requeue: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicRejectArguments {
    pub delivery_tag: u64,
    pub requeue: bool,
}

impl BasicRejectArguments {
    pub fn new(delivery_tag: u64) -> Self {
        Self {
            delivery_tag,
            requeue: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicPublishArguments {
    pub exchange: String,
    pub routing_key: String,
    pub mandatory: bool,
    pub immediate: bool,
}

impl BasicPublishArguments {
    pub fn new() -> Self {
        Self {
            exchange: "".to_string(),
            routing_key: "".to_string(),
            mandatory: false,
            immediate: false,
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
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

    /// return consumer tag
    pub async fn basic_consume<F>(
        &mut self,
        consumer: F,
        args: BasicConsumeArguments,
    ) -> Result<String>
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
            arguments.into_field_table(),
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
            trace!(
                "AsyncConsumer task starts for {} on channel {}!",
                ctag,
                channel.shared.channel_id
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
                        debug!("Exit consumer: {}", ctag);
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
        trace!("RegisterConsumer command is sent!");
        Ok(())
    }

    pub async fn basic_ack(&self, args: BasicAckArguments) -> Result<()> {
        let ack = Ack::new(args.delivery_tag, args.multiple);
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, ack.into_frame()))
            .await?;
        Ok(())
    }

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

    pub async fn basic_reject(&self, args: BasicRejectArguments) -> Result<()> {
        let reject = Reject::new(args.delivery_tag, args.requeue);
        self.shared
            .outgoing_tx
            .send((self.shared.channel_id, reject.into_frame()))
            .await?;
        Ok(())
    }

    pub async fn basic_cancel(&mut self, args: BasicCancelArguments) -> Result<String> {
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
        let cmd = UnregisterContentConsumer { consumer_tag };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::UnregisterContentConsumer(cmd))
            .await?;
        Ok(consumer_tag2)
    }

    pub async fn basic_get(&mut self, args: BasicGetArguments) -> Result<Option<GetMessage>> {
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
            Error::InternalChannelError("Failed to receive response to Get".to_string())
        })? {
            Frame::GetEmpty(_, _) => return Ok(None),
            Frame::GetOk(_, get_ok) => get_ok,
            _ => unreachable!("expect GetOk or GetEmpty"),
        };

        let basic_properties = match rx.recv().await.ok_or_else(|| {
            Error::InternalChannelError("Failed to receive Get ContentHeader".to_string())
        })? {
            Frame::ContentHeader(header) => header.basic_properties,
            _ => unreachable!("expect ContentHeader"),
        };

        let content = match rx.recv().await.ok_or_else(|| {
            Error::InternalChannelError("Failed to receive Get ContentBody".to_string())
        })? {
            Frame::ContentBody(content) => content.inner,
            _ => unreachable!("expect ContentBody"),
        };
        Ok(Some(GetMessage {
            get_ok,
            basic_properties,
            content,
        }))
    }

    /// RabbitMQ does not support `requeue = false`. User should always pass `true`.
    pub async fn basic_recover(&mut self, requeue: bool) -> Result<()> {
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

    /// TODO: add return call back
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

    use crate::{
        api::{
            channel::{QueueBindArguments, QueueDeclareArguments},
            connection::{Connection, OpenConnectionArguments},
            consumer::DefaultConsumer,
            Result
        },
        frame::BasicProperties,
    };

    use super::{BasicConsumeArguments, BasicPublishArguments};

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_basic_consume_auto_ack() {
        {
            let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");
            let client = Connection::open(&args).await.unwrap();

            let mut channel = client.open_channel().await.unwrap();
            channel
                .queue_declare(QueueDeclareArguments::new("amqprs"))
                .await
                .unwrap();
            channel
                .queue_bind(QueueBindArguments::new("amqprs", "amq.topic", "eiffel.#"))
                .await
                .unwrap();

            let mut args = BasicConsumeArguments::new();
            args.queue = "amqprs".to_string();
            args.consumer_tag = "tester".to_string();
            args.no_ack = true;
            channel
                .basic_consume(DefaultConsumer::new(args.no_ack), args)
                .await
                .unwrap();
            time::sleep(time::Duration::from_secs(15)).await;
        }

        println!("connection and channel are dropped");
        time::sleep(time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_basic_consume_manual_ack() {
        {
            let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

            let client = Connection::open(&args).await.unwrap();

            let mut channel = client.open_channel().await.unwrap();
            channel
                .queue_declare(QueueDeclareArguments::new("amqprs"))
                .await
                .unwrap();
            channel
                .queue_bind(QueueBindArguments::new("amqprs", "amq.topic", "eiffel.#"))
                .await
                .unwrap();

            let mut args = BasicConsumeArguments::new();
            args.queue = "amqprs".to_string();
            args.consumer_tag = "tester".to_string();
            channel
                .basic_consume(DefaultConsumer::new(args.no_ack), args)
                .await
                .unwrap();
            time::sleep(time::Duration::from_secs(15)).await;
        }

        println!("connection and channel are dropped");
        time::sleep(time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_basic_publish() {
        {
            let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

            let client = Connection::open(&args).await.unwrap();

            let channel = client.open_channel().await.unwrap();

            let mut args = BasicPublishArguments::new();
            args.exchange = "amq.topic".to_string();
            args.routing_key = "eiffel._.amqprs._.tester".to_string();

            let basic_properties = BasicProperties::new(
                Some(String::from("application/json;charset=utf-8")),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
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

        println!("connection and channel are dropped");
        time::sleep(time::Duration::from_secs(1)).await;
    }
}
