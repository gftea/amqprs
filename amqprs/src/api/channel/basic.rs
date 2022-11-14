use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    ops::Deref,
};

use tokio::{sync::mpsc, task::yield_now};

use crate::{
    api::{
        consumer::{self, Consumer},
        error::Error,
    },
    frame::{
        Ack, BasicProperties, Cancel, CancelOk, Consume, ConsumeOk, ContentBody, ContentHeader,
        ContentHeaderCommon, Deliver, Frame, Get, GetOk, Nack, Publish, Qos, QosOk, Recover,
        RecoverOk, Reject,
    },
    net::IncomingMessage,
};

use super::{Channel, Result, ServerSpecificArguments};

const CONSUMER_MESSAGE_BUFFER_SIZE: usize = 32;

#[derive(Debug)]
pub(crate) struct ConsumerMessage {
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

pub(crate) struct RegisterGetResponder {
    tx: mpsc::Sender<Frame>,
}
pub(crate) enum DispatcherManagementCommand {
    RegisterConsumer(RegisterConsumer),
    UnregisterConsumer(UnregisterConsumer),
    RegisterGetResponder(RegisterGetResponder),
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
    get_ok: GetOk,
    basic_properties: BasicProperties,
    content: Vec<u8>,
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

struct ConsumerResource {
    fifo: VecDeque<ConsumerMessage>,
    tx: Option<mpsc::Sender<ConsumerMessage>>,
}

impl ConsumerResource {
    fn new() -> Self {
        Self {
            fifo: VecDeque::new(),
            tx: None,
        }
    }
    fn register_tx(
        &mut self,
        tx: mpsc::Sender<ConsumerMessage>,
    ) -> Option<mpsc::Sender<ConsumerMessage>> {
        self.tx.replace(tx)
    }
    fn unregister_tx(&mut self) -> Option<mpsc::Sender<ConsumerMessage>> {
        self.tx.take()
    }

    fn get_tx(&self) -> Option<&mpsc::Sender<ConsumerMessage>> {
        self.tx.as_ref()
    }

    fn push(&mut self, message: ConsumerMessage) {
        self.fifo.push_back(message);
    }
    fn pop(&mut self) -> Option<ConsumerMessage> {
        self.fifo.pop_front()
    }
}
struct ConsumersResourcePool {
    inner: HashMap<String, ConsumerResource>,
}

impl ConsumersResourcePool {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
    fn get_or_new_consumer(&mut self, consumer_tag: &String) -> &mut ConsumerResource {
        if !self.inner.contains_key(consumer_tag) {
            let resource = ConsumerResource::new();
            self.inner.insert(consumer_tag.clone(), resource);
        }
        self.inner.get_mut(consumer_tag).unwrap()
    }

    fn remove_consumer(&mut self, consumer_tag: &String) -> Option<ConsumerResource> {
        self.inner.remove(consumer_tag)
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    /// Dispatcher for content related frames
    pub(in crate::api) async fn spawn_dispatcher(
        &self,
        mut dispatcher_rx: mpsc::Receiver<Frame>,
        mut dispatcher_mgmt_rx: mpsc::Receiver<DispatcherManagementCommand>,
    ) {
        let channel_id = self.channel_id;

        tokio::spawn(async move {
            let mut buffer = ConsumerMessage {
                deliver: None,
                basic_properties: None,
                content: None,
            };
            let mut consumers = ConsumersResourcePool::new();
            let mut sync_responder = None;
            // internal state
            enum State {
                Initial,
                Deliver,
                GetOk,
                GetEmpty,
                Return,
            }
            // initial state
            let mut state = State::Initial;
            println!("Dispatcher of channel {} starts!", channel_id);

            loop {
                tokio::select! {
                    biased;

                    command = dispatcher_mgmt_rx.recv() => {
                        let cmd = match command {
                            None => break,
                            Some(v) => v,
                        };
                        match cmd {
                            DispatcherManagementCommand::RegisterConsumer(cmd) => {
                                // TODO: check insert result
                                println!("Consumer: {}, tx registered!", cmd.consumer_tag);
                                let consumer = consumers.get_or_new_consumer(&cmd.consumer_tag);
                                consumer.register_tx(cmd.consumer_tx);
                                // forward buffered messages
                                while !consumer.fifo.is_empty() {
                                    println!("Total buffered messages: {}", consumer.fifo.len());
                                    let msg = consumer.pop().unwrap();
                                    consumer.get_tx().unwrap().send(msg).await.unwrap();
                                }

                            },
                            DispatcherManagementCommand::UnregisterConsumer(cmd) => {
                                // TODO: check remove result
                                consumers.remove_consumer(&cmd.consumer_tag);

                            },
                            DispatcherManagementCommand::RegisterGetResponder(cmd) => {
                                sync_responder = Some(cmd.tx);
                            }
                        }
                    }
                    frame = dispatcher_rx.recv() => {
                        let frame = match frame {
                            None => break,
                            Some(v) => v,
                        };
                        match frame {
                            Frame::Return(_, method) => {
                                state = State::Return;
                                println!("returned : {}, {}", method.reply_code, method.reply_text.deref());
                            }
                            Frame::GetEmpty(_, get_empty) => {
                                state = State::GetEmpty;
                                if let Err(err) = sync_responder.take().expect("Get responder must be registered").send(get_empty.into_frame()).await {
                                    println!("Failed to dispatch GetEmpty frame, cause: {}", err);
                                }
                            }
                            Frame::GetOk(_, get_ok) => {
                                state = State::GetOk;
                                if let Err(err) = sync_responder.as_ref().expect("Get responder must be registered").send(get_ok.into_frame()).await {
                                    println!("Failed to dispatch GetOk frame, cause: {}", err);
                                }
                            }
                            // server must send "Deliver + Content" in order, otherwise
                            // client cannot know to which consumer tag is the content frame
                            Frame::Deliver(_, deliver) => {

                                state = State::Deliver;
                                buffer.deliver = Some(deliver);
                            }
                            Frame::ContentHeader(header) => {
                                match state {
                                    State::Deliver => buffer.basic_properties = Some(header.basic_properties),
                                    State::GetOk => {
                                        if let Err(err) = sync_responder.as_ref().expect("Get responder must be registered").send(header.into_frame()).await {
                                            println!("Failed to dispatch GetOk ContentHeader frame, cause: {}", err);
                                        }
                                    },
                                    State::Return => todo!("handle Return content"),
                                    State::Initial | State::GetEmpty  => unreachable!("invalid dispatcher state"),
                                }

                            }
                            Frame::ContentBody(body) => {
                                match state {
                                    State::Deliver => {
                                        buffer.content = Some(body.inner);

                                        let consumer_tag = buffer.deliver.as_ref().unwrap().consumer_tag().clone();
                                        let consumer_message  = ConsumerMessage {
                                            deliver: buffer.deliver.take(),
                                            basic_properties: buffer.basic_properties.take(),
                                            content: buffer.content.take(),
                                        };
                                        let consumer = consumers.get_or_new_consumer(&consumer_tag);
                                        match consumer.get_tx() {
                                            Some(consumer_tx) => {
                                                if let Err(_) = consumer_tx.send(consumer_message).await {
                                                    println!("Failed to dispatch message to consumer {}", consumer_tag);
                                                }
                                            },
                                            None => {
                                                println!("Can't find consumer '{}', buffering message", consumer_tag);
                                                consumer.push(consumer_message);
                                                // FIXME: try to yield for registering consumer
                                                //      not sure if it is necessary
                                                yield_now().await;
                                            },
                                        };
                                    }
                                    State::GetOk => {
                                        if let Err(err) = sync_responder.take().expect("Get responder must be registered").send(body.into_frame()).await {
                                            println!("Failed to dispatch GetOk ContentBody frame, cause: {}", err);
                                        }
                                    },
                                    State::Return => todo!("handle Return content"),
                                    State::Initial | State::GetEmpty  => unreachable!("invalid dispatcher state"),
                                }

                            }
                            _ => unreachable!("Not acceptable frame for dispatcher: {:?}", frame),
                        }
                    }
                    else => {
                        break;
                    }

                }
            }
            println!("Exit dispatcher of channel {}", channel_id);
        });
    }

    pub async fn basic_qos(&mut self, args: BasicQosArguments) -> Result<()> {
        let qos = Qos {
            prefetch_size: args.prefetch_size,
            prefetch_count: args.prefetch_count,
            global: args.global,
        };
        let responder_rx = self.register_responder(QosOk::header()).await?;

        let _method = synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, qos.into_frame()),
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
            let responder_rx = self.register_responder(ConsumeOk::header()).await?;

            let method = synchronous_request!(
                self.outgoing_tx,
                (self.channel_id, consume.into_frame()),
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
        F: Consumer + Send + 'static,
    {
        let (consumer_tx, mut consumer_rx): (
            mpsc::Sender<ConsumerMessage>,
            mpsc::Receiver<ConsumerMessage>,
        ) = mpsc::channel(CONSUMER_MESSAGE_BUFFER_SIZE);

        let ctag = consumer_tag.clone();
        let channel = self.clone();
        // spawn consumer task
        tokio::spawn(async move {
            println!(
                "Consumer task starts for {} on channel {}!",
                ctag, channel.channel_id
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
                        println!("Exit consumer: {}", ctag);
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
        println!("RegisterConsumer command is sent!");
        Ok(())
    }

    pub async fn basic_ack(&self, args: BasicAckArguments) -> Result<()> {
        let ack = Ack {
            delivery_tag: args.delivery_tag,
            mutiple: args.multiple,
        };
        self.outgoing_tx
            .send((self.channel_id, ack.into_frame()))
            .await?;
        Ok(())
    }

    pub async fn basic_nack(&self, args: BasicNackArguments) -> Result<()> {
        let mut nack = Nack {
            delivery_tag: args.delivery_tag,
            bits: 0,
        };
        nack.set_multiple(args.multiple);
        nack.set_requeue(args.requeue);
        self.outgoing_tx
            .send((self.channel_id, nack.into_frame()))
            .await?;
        Ok(())
    }

    pub async fn basic_reject(&self, args: BasicRejectArguments) -> Result<()> {
        let reject = Reject {
            delivery_tag: args.delivery_tag,
            requeue: args.requeue,
        };
        self.outgoing_tx
            .send((self.channel_id, reject.into_frame()))
            .await?;
        Ok(())
    }

    pub async fn basic_cancel(&mut self, args: BasicCancelArguments) -> Result<String> {
        let BasicCancelArguments {
            consumer_tag,
            no_wait,
        } = args;
        let cancel = Cancel {
            consumer_tag: consumer_tag.clone().try_into().unwrap(),
            no_wait,
        };
        let consumer_tag = if args.no_wait {
            self.outgoing_tx
                .send((self.channel_id, cancel.into_frame()))
                .await?;
            consumer_tag
        } else {
            let responder_rx = self.register_responder(CancelOk::header()).await?;

            let method = synchronous_request!(
                self.outgoing_tx,
                (self.channel_id, cancel.into_frame()),
                responder_rx,
                Frame::CancelOk,
                Error::ChannelUseError
            )?;
            method.consumer_tag.into()
        };
        // FIXME: haven't unregister consumer in dispatcher
        //  because there can be buffered messages to be handled
        Ok(consumer_tag)
    }

    pub async fn basic_get(&mut self, args: BasicGetArguments) -> Result<Option<GetMessage>> {
        let get = Get {
            ticket: 0,
            queue: args.queue.try_into().unwrap(),
            no_ack: args.no_ack,
        };

        let (tx, mut rx) = mpsc::channel(3);
        let command = RegisterGetResponder { tx };
        self.dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterGetResponder(command))
            .await?;

        self.outgoing_tx
            .send((self.channel_id, get.into_frame()))
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
        let recover = Recover { requeue };

        let responder_rx = self.register_responder(RecoverOk::header()).await?;

        let _method = synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, recover.into_frame()),
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
            let client = Connection::open("localhost:5672").await.unwrap();

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
