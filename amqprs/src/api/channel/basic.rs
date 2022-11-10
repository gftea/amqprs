use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc;

use crate::{
    api::{consumer::Consumer, error::Error},
    frame::{
        Ack, BasicProperties, Consume, ContentBody, ContentHeader, ContentHeaderCommon, Deliver,
        Frame, Nack, Publish, Qos, Recover, Reject,
    },
};

use super::{Acker, Channel, Result, ServerSpecificArguments};
pub(crate) type SharedConsumerQueue =
    Arc<Mutex<BTreeMap<String, (Box<dyn Consumer + Send>, Option<Acker>)>>>;

const CONSUMER_MESSAGE_BUFFER_SIZE: usize = 32;

#[derive(Debug)]
struct ConsumerMessage {
    deliver: Option<Deliver>,
    basic_properties: Option<BasicProperties>,
    content: Option<Vec<u8>>,
}
pub(crate) struct RegisterConsumer {
    consumer_tag: String,
    consumer_tx: mpsc::Sender<ConsumerMessage>,
}
pub(crate) struct UnregisterConsumer {
    consumer_tag: String,
}
pub(crate) enum DispatcherManagementCommand {
    RegisterConsumer(RegisterConsumer),
    UnregisterConsumer(UnregisterConsumer),
}
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

#[derive(Debug)]
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
    pub(in crate::api) async fn spawn_dispatcher(&mut self) {
        let channel_id = self.channel_id;
        let mut consumers = BTreeMap::new();

        let mut dispatcher_rx = self.dispatcher_rx.take().unwrap();
        let mut dispatcher_mgmt_rx = self.dispatcher_mgmt_rx.take().unwrap();

        tokio::spawn(async move {
            let mut buffer = ConsumerMessage {
                deliver: None,
                basic_properties: None,
                content: None,
            };
            // beginning state
            let unparked = true;
            loop {
                tokio::select! {
                    biased;

                    Some(cmd) = dispatcher_mgmt_rx.recv() => {
                        match cmd {
                            DispatcherManagementCommand::RegisterConsumer(cmd) => {
                                // TODO: check insert result
                                println!("consumer inserted for {}", cmd.consumer_tag);
                                consumers.insert(cmd.consumer_tag, cmd.consumer_tx);

                            },
                            DispatcherManagementCommand::UnregisterConsumer(cmd) => {
                                // TODO: check remove result
                                consumers.remove(&cmd.consumer_tag);

                            },
                        }
                    }
                    Some(frame) = dispatcher_rx.recv() => {
                        match frame {
                            // server must send "Deliver + Content" in order, otherwise
                            // client cannot know to which consumer tag is the content frame
                            Frame::Deliver(_, deliver) => {

                                buffer.deliver = Some(deliver);
                            }
                            Frame::ContentHeader(header) => {
                                buffer.basic_properties = Some(header.basic_properties);
                            }
                            Frame::ContentBody(body) => {
                                buffer.content = Some(body.inner);
                                // let deliver = message.deliver.take().unwrap();
                                // let basic_properties = message.basic_properties.take().unwrap();
                                // {
                                //     let k = deliver.consumer_tag().clone();
                                //     // lock and get consumer
                                //     let (mut consumer, acker) =
                                //         if let Some(v) = consumer_queue.lock().unwrap().remove(&k) {
                                //             v
                                //         } else {
                                //             println!(
                                //                 "ignore message because consumer is not registered yet"
                                //             );
                                //             continue;
                                //         };

                                //     consumer
                                //         .consume(
                                //             acker.as_ref(),
                                //             deliver,
                                //             basic_properties,
                                //             body.inner,
                                //         )
                                //         .await;

                                //     // lock to restore consumer
                                //     consumer_queue.lock().unwrap().insert(k, (consumer, acker));
                                // }
                                let consumer_tag = buffer.deliver.as_ref().unwrap().consumer_tag().clone();

                                match consumers.get(&consumer_tag) {
                                    Some(consumer_tx) => {
                                        let consumer_message  = ConsumerMessage {
                                            deliver: buffer.deliver.take(),
                                            basic_properties: buffer.basic_properties.take(),
                                            content: buffer.content.take(),
                                        };
                                        if let Err(_) = consumer_tx.send(consumer_message).await {
                                            println!("failed to dispatch message to consumer {}", consumer_tag);
                                        }
                                    },
                                    None => {
                                        println!("can't find consumer '{}', ignore message", consumer_tag);
                                    },
                                };

                            }
                            _ => unreachable!("not acceptable frame for dispatcher: {:?}", frame),
                        }

                    }
                    else => {
                        println!("exit dispatcher of channel {}", channel_id);
                        break;
                    }

                }
            }
        });
    }

    pub async fn basic_qos(&mut self, args: BasicQosArguments) -> Result<()> {
        let qos = Qos {
            prefetch_size: args.prefetch_size,
            prefetch_count: args.prefetch_count,
            global: args.global,
        };
        let _method = synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, qos.into_frame()),
            self.incoming_rx,
            Frame::QosOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    /// return consumer tag
    pub async fn basic_consume<F>(
        &mut self,
        args: BasicConsumeArguments,
        consumer: F,
    ) -> Result<String>
    where
        // TODO: this is blocking callback, spawn blocking task in connection manager
        // to provide async callback, and spawn async task  in connection manager
        F: Consumer + Send + 'static,
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

        let mut consume = Consume {
            ticket: 0,
            queue: queue.try_into().unwrap(),
            consumer_tag: consumer_tag.clone().try_into().unwrap(),
            bits: 0,
            arguments: arguments.into_field_table(),
        };
        consume.set_no_local(no_local);
        consume.set_no_ack(no_ack);
        consume.set_exclusive(exclusive);
        consume.set_nowait(no_wait);

        // before start consume, park the dispatcher first,
        // unpark the dispatcher after we have added consumer into callback queue
        // self.park_notify.notify_one();

        let consumer_tag = if args.no_wait {
            self.outgoing_tx
                .send((self.channel_id, consume.into_frame()))
                .await?;
            consumer_tag
        } else {
            let method = synchronous_request!(
                self.outgoing_tx,
                (self.channel_id, consume.into_frame()),
                self.incoming_rx,
                Frame::ConsumeOk,
                Error::ChannelUseError
            )?;
            method.consumer_tag.into()
        };

        self.spawn_consumer(no_ack, consumer_tag.clone(), consumer)
            .await?;

        Ok(consumer_tag)
    }

    async fn spawn_consumer<F>(
        &self,
        no_ack: bool,
        consumer_tag: String,
        mut consumer: F,
    ) -> Result<()>
    where
        F: Consumer + Send + 'static,
    {
        let (consumer_tx, mut consumer_rx): (
            mpsc::Sender<ConsumerMessage>,
            mpsc::Receiver<ConsumerMessage>,
        ) = mpsc::channel(CONSUMER_MESSAGE_BUFFER_SIZE);

        let acker = if no_ack { None } else { Some(self.new_acker()) };
        let ctag = consumer_tag.clone();
        tokio::spawn(async move {
            // let acker = acker.as_ref();
            loop {
                match consumer_rx.recv().await {
                    Some(mut msg) => {
                        consumer
                            .consume(
                                acker.as_ref(),
                                msg.deliver.take().unwrap(),
                                msg.basic_properties.take().unwrap(),
                                msg.content.take().unwrap(),
                            )
                            .await;
                    }
                    None => {
                        println!("exit consumer: {}", ctag);
                        break;
                    }
                }
            }
        });
        self.dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterConsumer(
                RegisterConsumer {
                    consumer_tag,
                    consumer_tx,
                },
            ))
            .await?;
        Ok(())
    }

    pub fn new_acker(&self) -> Acker {
        Acker {
            tx: self.outgoing_tx.clone(),
            channel_id: self.channel_id,
        }
    }

    pub async fn basic_cancel(&mut self) {
        todo!()
    }
    pub async fn basic_get(&mut self) {
        todo!()
    }

    /// RabbitMQ does not support `requeue = false`. User should always pass `true`.
    pub async fn basic_recover(&mut self, requeue: bool) -> Result<()> {
        let recover = Recover { requeue };

        let _method = synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, recover.into_frame()),
            self.incoming_rx,
            Frame::RecoverOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    /// TODO: add return call back
    pub async fn basic_publish(
        &self,
        args: BasicPublishArguments,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) -> Result<()> {
        let mut publish = Publish {
            ticket: 0,
            exchange: args.exchange.try_into().unwrap(),
            routing_key: args.routing_key.try_into().unwrap(),
            bits: 0,
        };
        publish.set_mandatory(args.mandatory);
        publish.set_immediate(args.immediate);

        self.outgoing_tx
            .send((self.channel_id, publish.into_frame()))
            .await?;

        let content_header = ContentHeader::new(
            ContentHeaderCommon {
                class: 60, // basic class
                weight: 0,
                body_size: content.len() as u64,
            },
            basic_properties,
        );
        self.outgoing_tx
            .send((self.channel_id, content_header.into_frame()))
            .await?;

        let content = ContentBody::new(content);
        self.outgoing_tx
            .send((self.channel_id, content.into_frame()))
            .await?;
        Ok(())
    }
}

impl Acker {
    pub async fn basic_ack(&self, args: BasicAckArguments) -> Result<()> {
        let ack = Ack {
            delivery_tag: args.delivery_tag,
            mutiple: args.multiple,
        };
        self.tx.send((self.channel_id, ack.into_frame())).await?;
        Ok(())
    }
    pub async fn basic_nack(&self, args: BasicNackArguments) -> Result<()> {
        let mut nack = Nack {
            delivery_tag: args.delivery_tag,
            bits: 0,
        };
        nack.set_multiple(args.multiple);
        nack.set_requeue(args.requeue);
        self.tx.send((self.channel_id, nack.into_frame())).await?;
        Ok(())
    }
    pub async fn basic_reject(&self, args: BasicRejectArguments) -> Result<()> {
        let reject = Reject {
            delivery_tag: args.delivery_tag,
            requeue: args.requeue,
        };
        self.tx.send((self.channel_id, reject.into_frame())).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::time;

    use crate::{
        api::{
            channel::{QueueBindArguments, QueueDeclareArguments},
            connection::Connection,
            consumer::DefaultConsumer,
        },
        frame::BasicProperties,
    };

    use super::{BasicConsumeArguments, BasicPublishArguments};

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_basic_consume_auto_ack() {
        {
            let client = Connection::open("localhost:5672").await.unwrap();

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
            channel.basic_consume(args, DefaultConsumer).await.unwrap();
            time::sleep(time::Duration::from_secs(15)).await;
        }
        time::sleep(time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_basic_consume_manual_ack() {
        {
            let client = Connection::open("localhost:5672").await.unwrap();

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
            channel.basic_consume(args, DefaultConsumer).await.unwrap();
            time::sleep(time::Duration::from_secs(15)).await;
        }
        time::sleep(time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_basic_publish() {
        {
            let client = Connection::open("localhost:5672").await.unwrap();

            let mut channel = client.open_channel().await.unwrap();

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
                .basic_publish(args, basic_properties, content)
                .await
                .unwrap();
            time::sleep(time::Duration::from_secs(1)).await;
        }
        time::sleep(time::Duration::from_secs(1)).await;
    }
}
