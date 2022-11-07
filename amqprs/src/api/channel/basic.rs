use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use amqp_serde::types::AmqpChannelId;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::{
    api::{consumer::Consumer, error::Error},
    frame::{
        Ack, BasicProperties, Consume, ContentBody, ContentHeader, ContentHeaderCommon, Deliver,
        Frame, Publish, Qos,
    },
};

use super::{Acker, Channel, Result, ServerSpecificArguments};
pub(crate) type SharedConsumerQueue =
    Arc<Mutex<BTreeMap<String, (Box<dyn Consumer + Send>, Option<Acker>)>>>;

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
        let consumer_queue = self.consumer_queue.clone();

        let mut dispatcher_rx = self.dispatcher_rx.take().unwrap();

        tokio::spawn(async move {
            #[derive(Debug)]
            struct ConsumerMessage {
                deliver: Option<Deliver>,
                basic_propertities: Option<BasicProperties>,
            }
            let mut message = ConsumerMessage {
                deliver: None,
                basic_propertities: None,
            };
            loop {
                match dispatcher_rx.recv().await {
                    None => {
                        println!("exit dispatcher of channel: {}", channel_id);
                        break;
                    }
                    Some(frame) => match frame {
                        Frame::Deliver(_, deliver) => {
                            message.deliver = Some(deliver);
                        }
                        Frame::ContentHeader(header) => {
                            message.basic_propertities = Some(header.basic_properties);
                        }
                        Frame::ContentBody(body) => {
                            let deliver = message.deliver.take().unwrap();
                            let basic_propertities = message.basic_propertities.take().unwrap();
                            // let delivery_tag = deliver.delivery_tag();
                            {
                                let k = deliver.consumer_tag().clone();
                                // lock and get consumer
                                let (mut consumer, acker) =
                                    if let Some(v) = consumer_queue.lock().unwrap().remove(&k) {
                                        v
                                    } else {
                                        println!(
                                            "ignore message because consumer is not registered yet"
                                        );
                                        return;
                                    };

                                consumer
                                    .consume(
                                        acker.as_ref(),
                                        deliver,
                                        basic_propertities,
                                        body.inner,
                                    )
                                    .await;

                                // lock to restore consumer
                                consumer_queue.lock().unwrap().insert(k, (consumer, acker));
                            }
                        }
                        _ => unreachable!("not acceptable frame for dispatcher: {:?}", frame),
                    },
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

        // TODO: spawn task for consumer and register consumer to dispatcher
        // ReaderHandler forward message to dispatcher, dispatcher forward message to or invovke consumer with message
        // Edge case:
        //  dispatcher may be scheduled immediately before consumer is inserted,
        //  which used to happen when publisher start first
        self.consumer_queue.lock().unwrap().insert(
            consumer_tag.clone(),
            if no_ack {
                (Box::new(consumer), None)
            } else {
                (Box::new(consumer), Some(self.new_acker()))
            },
        );
        println!("consumer inserted for {}", consumer_tag);
        self.spawn_dispatcher().await;
        Ok(consumer_tag)
    }

    // TODO:
    // alt1:
    //  one task per consumer, and  one task for dispatcher per channel,
    //  a dispatcher distribute message to different consumer tasks.
    // alt2:
    //  it just use callback queue, each channel has only one task for all consumers
    //  the task call the callback - Q: how to insert callback lock-free?
    async fn spawn_consumer(&self) {}

    pub fn new_acker(&self) -> Acker {
        Acker {
            tx: self.outgoing_tx.clone(),
            channel_id: self.channel_id,
        }
    }
    // pub async fn basic_ack(&self, args: BasicAckArguments) -> Result<()> {
    //     let ack = Ack {
    //         delivery_tag: args.delivery_tag,
    //         mutiple: args.multiple,
    //     };
    //     self.outgoing_tx
    //         .send((self.channel_id, ack.into_frame()))
    //         .await?;
    //     Ok(())
    // }

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
        frame::{BasicProperties, ContentBody},
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
