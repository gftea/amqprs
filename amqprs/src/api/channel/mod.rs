//! API implementation of AMQP Channel
//!

use std::{
    collections::{HashMap, VecDeque},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use amqp_serde::types::{AmqpChannelId, FieldTable, FieldValue};
use tokio::{
    sync::{mpsc, oneshot},
    task::yield_now,
};

use crate::{
    api::error::Error,
    frame::{CloseChannel, CloseChannelOk, Deliver, Flow, FlowOk, Frame, MethodHeader},
    net::{ConnManagementCommand, IncomingMessage, OutgoingMessage},
    BasicProperties,
};
use tracing::{debug, error, trace};

type Result<T> = std::result::Result<T, Error>;

pub(crate) const CONSUMER_MESSAGE_BUFFER_SIZE: usize = 32;

#[derive(Debug)]
pub(crate) struct ConsumerMessage {
    deliver: Option<Deliver>,
    basic_properties: Option<BasicProperties>,
    content: Option<Vec<u8>>,
}

pub(crate) struct RegisterContentConsumer {
    consumer_tag: String,
    consumer_tx: mpsc::Sender<ConsumerMessage>,
}
pub(crate) struct UnregisterContentConsumer {
    consumer_tag: String,
}

pub(crate) struct RegisterGetContentResponder {
    tx: mpsc::Sender<IncomingMessage>,
}

pub(crate) struct RegisterOneshotResponder {
    pub method_header: &'static MethodHeader,
    pub responder: oneshot::Sender<IncomingMessage>,
    pub acker: oneshot::Sender<()>,
}

pub(crate) struct RegisterChannelCallback {
    pub callback: Box<dyn ChannelCallback + Send + 'static>,
}
pub(crate) enum DispatcherManagementCommand {
    RegisterContentConsumer(RegisterContentConsumer),
    UnregisterContentConsumer(UnregisterContentConsumer),
    RegisterGetContentResponder(RegisterGetContentResponder),
    RegisterOneshotResponder(RegisterOneshotResponder),
    RegisterChannelCallback(RegisterChannelCallback),
}

struct ConsumerBuffer {
    fifo: VecDeque<ConsumerMessage>,
    tx: Option<mpsc::Sender<ConsumerMessage>>,
}

impl ConsumerBuffer {
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
struct ConsumerBuffersPool {
    inner: HashMap<String, ConsumerBuffer>,
}

impl ConsumerBuffersPool {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
    fn get_or_new_consumer(&mut self, consumer_tag: &String) -> &mut ConsumerBuffer {
        if !self.inner.contains_key(consumer_tag) {
            let resource = ConsumerBuffer::new();
            self.inner.insert(consumer_tag.clone(), resource);
        }
        self.inner.get_mut(consumer_tag).unwrap()
    }

    fn remove_consumer(&mut self, consumer_tag: &String) -> Option<ConsumerBuffer> {
        self.inner.remove(consumer_tag)
    }
}

/// Represent an AMQP Channel.
///
/// To create a AMQP channel, use [`Connection::channel` method][`channel`]
///
/// [`channel`]: crate::api::connection::Connection::channel
#[derive(Debug, Clone)]
pub struct Channel {
    shared: Arc<SharedChannelInner>,
}

#[derive(Debug)]
pub(crate) struct SharedChannelInner {
    is_open: AtomicBool,

    channel_id: AmqpChannelId,

    outgoing_tx: mpsc::Sender<OutgoingMessage>,

    conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,

    dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
}

impl SharedChannelInner {
    pub(in crate::api) fn new(
        is_open: AtomicBool,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            is_open,
            channel_id,
            outgoing_tx,
            conn_mgmt_tx,
            dispatcher_mgmt_tx,
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    pub(in crate::api) fn new(shared: Arc<SharedChannelInner>) -> Self {
        Self { shared }
    }
    pub(in crate::api) async fn spawn_dispatcher(
        &self,
        mut dispatcher_rx: mpsc::Receiver<IncomingMessage>,
        mut dispatcher_mgmt_rx: mpsc::Receiver<DispatcherManagementCommand>,
    ) {
        let channel_id = self.shared.channel_id;
        let channel = self.clone();

        tokio::spawn(async move {
            // internal state
            enum State {
                Initial,
                Deliver,
                GetOk,
                GetEmpty,
                Return,
            }
            let channel = &channel;
            // buffer pool for all consumers
            let mut consumers = ConsumerBuffersPool::new();
            // single message buffer
            let mut message_buffer = ConsumerMessage {
                deliver: None,
                basic_properties: None,
                content: None,
            };
            // responders for Get content and synchronous response
            let mut get_responder = None;
            let mut oneshot_responders: HashMap<
                &'static MethodHeader,
                oneshot::Sender<IncomingMessage>,
            > = HashMap::new();

            let mut callback = None;

            // initial state
            let mut state = State::Initial;

            trace!("Dispatcher of channel {} starts!", channel_id);

            loop {
                tokio::select! {
                    biased;

                    command = dispatcher_mgmt_rx.recv() => {
                        // handle command channel error
                        let cmd = match command {
                            None => break,
                            Some(v) => v,
                        };
                        // handle command
                        match cmd {
                            DispatcherManagementCommand::RegisterContentConsumer(cmd) => {
                                // TODO: check insert result
                                trace!("AsyncConsumer: {}, tx registered!", cmd.consumer_tag);
                                let consumer = consumers.get_or_new_consumer(&cmd.consumer_tag);
                                consumer.register_tx(cmd.consumer_tx);
                                // forward buffered messages
                                while !consumer.fifo.is_empty() {
                                    trace!("Total buffered messages: {}", consumer.fifo.len());
                                    let msg = consumer.pop().unwrap();
                                    consumer.get_tx().unwrap().send(msg).await.unwrap();
                                }

                            },
                            DispatcherManagementCommand::UnregisterContentConsumer(cmd) => {
                                // TODO: check remove result
                                consumers.remove_consumer(&cmd.consumer_tag);

                            },
                            DispatcherManagementCommand::RegisterGetContentResponder(cmd) => {
                                get_responder.replace(cmd.tx);
                            }
                            DispatcherManagementCommand::RegisterOneshotResponder(cmd) => {
                                oneshot_responders.insert(cmd.method_header, cmd.responder);
                                cmd.acker.send(()).unwrap();
                            }
                            DispatcherManagementCommand::RegisterChannelCallback(cmd) => {
                                callback.replace(cmd.callback);
                            }
                        }
                    }
                    message = dispatcher_rx.recv() => {
                        // handle message channel error
                        let frame = match message {
                            None => break,
                            Some(v) => v,
                        };
                        // handle frames
                        match frame {
                            Frame::Return(_, method) => {
                                state = State::Return;
                                debug!("returned : {}, {}", method.reply_code, method.reply_text.deref());
                            }
                            Frame::GetEmpty(_, get_empty) => {
                                state = State::GetEmpty;
                                if let Err(err) = get_responder.take().expect("Get responder must be registered").send(get_empty.into_frame()).await {
                                    debug!("Failed to dispatch GetEmpty frame, cause: {}", err);
                                }
                            }
                            Frame::GetOk(_, get_ok) => {
                                state = State::GetOk;
                                if let Err(err) = get_responder.as_ref().expect("Get responder must be registered").send(get_ok.into_frame()).await {
                                    debug!("Failed to dispatch GetOk frame, cause: {}", err);
                                }
                            }
                            Frame::Deliver(_, deliver) => {

                                state = State::Deliver;
                                message_buffer.deliver = Some(deliver);
                            }
                            Frame::ContentHeader(header) => {
                                match state {
                                    State::Deliver => message_buffer.basic_properties = Some(header.basic_properties),
                                    State::GetOk => {
                                        if let Err(err) = get_responder.as_ref().expect("Get responder must be registered").send(header.into_frame()).await {
                                            debug!("Failed to dispatch GetOk ContentHeader frame, cause: {}", err);
                                        }
                                    },
                                    State::Return => todo!("handle Return content"),
                                    State::Initial | State::GetEmpty  => unreachable!("invalid dispatcher state"),
                                }

                            }
                            Frame::ContentBody(body) => {
                                match state {
                                    State::Deliver => {
                                        message_buffer.content = Some(body.inner);

                                        let consumer_tag = message_buffer.deliver.as_ref().unwrap().consumer_tag().clone();
                                        let consumer_message  = ConsumerMessage {
                                            deliver: message_buffer.deliver.take(),
                                            basic_properties: message_buffer.basic_properties.take(),
                                            content: message_buffer.content.take(),
                                        };
                                        let consumer = consumers.get_or_new_consumer(&consumer_tag);
                                        match consumer.get_tx() {
                                            Some(consumer_tx) => {
                                                if let Err(_) = consumer_tx.send(consumer_message).await {
                                                    debug!("Failed to dispatch message to consumer {}", consumer_tag);
                                                }
                                            },
                                            None => {
                                                debug!("Can't find consumer '{}', buffering message", consumer_tag);
                                                consumer.push(consumer_message);
                                                // FIXME: try to yield for registering consumer
                                                //      not sure if it is necessary
                                                yield_now().await;
                                            },
                                        };
                                    }
                                    State::GetOk => {
                                        if let Err(err) = get_responder.take().expect("Get responder must be registered").send(body.into_frame()).await {
                                            debug!("Failed to dispatch GetOk ContentBody frame, cause: {}", err);
                                        }
                                    },
                                    State::Return => todo!("handle Return content"),
                                    State::Initial | State::GetEmpty  => unreachable!("invalid dispatcher state"),
                                }


                            }
                            // Close channel response from server
                            Frame::CloseChannelOk(method_header, close_channel_ok) => {
                                oneshot_responders.remove(method_header)
                                .expect("CloseChannelOk responder must be registered")
                                .send(close_channel_ok.into_frame()).unwrap();

                                channel.set_open_state(false);
                                break;
                            }
                            // TODO:
                            | Frame::FlowOk(method_header, _)
                            | Frame::RequestOk(method_header, _) // deprecated
                            | Frame::DeclareOk(method_header, _)
                            | Frame::DeleteOk(method_header, _)
                            | Frame::BindOk(method_header, _)
                            | Frame::UnbindOk(method_header, _)
                            | Frame::DeclareQueueOk(method_header, _)
                            | Frame::BindQueueOk(method_header, _)
                            | Frame::PurgeQueueOk(method_header, _)
                            | Frame::DeleteQueueOk(method_header, _)
                            | Frame::UnbindQueueOk(method_header, _)
                            | Frame::QosOk(method_header, _)
                            | Frame::ConsumeOk(method_header, _)
                            | Frame::CancelOk(method_header, _)
                            | Frame::RecoverOk(method_header, _)
                            | Frame::SelectOk(method_header, _)
                            | Frame::SelectTxOk(method_header, _)
                            | Frame::CommitOk(method_header, _)
                            | Frame::RollbackOk(method_header, _) => {
                                // handle synchronous response
                                match oneshot_responders.remove(method_header)
                                {
                                    Some(responder) => {
                                        if let Err(response) = responder.send(frame) {
                                            debug!(
                                                "Failed to forward response frame {} to channel {}",
                                                response, channel_id
                                            );
                                        }
                                    }
                                    None => debug!(
                                        "No responder to forward frame {} to channel {}",
                                        frame, channel_id
                                    ),
                                }

                            }
                            //////////////////////////////////////////////////////////
                            // Method frames of asynchronous request

                            // Server request to close channel
                            Frame::CloseChannel(_method_header, close_channel) => {
                                channel.set_open_state(false);
                                // first, respond to server that we have received the request
                                channel.shared.outgoing_tx
                                .send((channel_id, CloseChannelOk::default().into_frame()))
                                .await.unwrap();

                                // callback
                                if let Some(mut cb) = callback {
                                    cb.close(channel, close_channel).await.unwrap();
                                }
                                break;
                            }
                            // TODO
                            | Frame::Flow(_method_header, _)
                            | Frame::Cancel(_method_header, _)
                            | Frame::Ack(_method_header, _) // confirmed mode
                            | Frame::Nack(_method_header, _) => {
                                todo!("handle asynchronous request")
                            }
                            _ => unreachable!("Not acceptable frame for dispatcher: {:?}", frame),
                        }
                    }
                    else => {
                        break;
                    }

                }
            }
            trace!("Exit dispatcher of channel {}", channel_id);
        });
    }

    async fn handle_frame(&mut self, frame: Frame) -> Result<()> {
        match frame {
            unexpected => unreachable!("unexpected frames {}", unexpected),
        }
    }
    pub async fn register_callback<F>(&self, callback: F) -> Result<()>
    where
        F: ChannelCallback + Send + 'static,
    {
        let cmd = RegisterChannelCallback {
            callback: Box::new(callback),
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterChannelCallback(cmd))
            .await?;
        Ok(())
    }
    async fn register_responder(
        &self,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<IncomingMessage>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterOneshotResponder {
            method_header,
            responder,
            acker,
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterOneshotResponder(cmd))
            .await?;
        acker_rx.await?;
        Ok(responder_rx)
    }

    ///
    pub async fn flow(&mut self, active: bool) -> Result<()> {
        let responder_rx = self.register_responder(FlowOk::header()).await?;
        synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, Flow { active }.into_frame()),
            responder_rx,
            Frame::FlowOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }
    pub fn is_connection_closed(&self) -> bool {
        self.shared.conn_mgmt_tx.is_closed()
    }

    pub fn channel_id(&self) -> AmqpChannelId {
        self.shared.channel_id
    }
    pub(crate) fn set_open_state(&self, is_open: bool) {
        self.shared.is_open.store(is_open, Ordering::Relaxed);
    }
    pub fn get_open_state(&self) -> bool {
        self.shared.is_open.load(Ordering::Relaxed)
    }
    /// User must close the channel to avoid channel leak
    pub async fn close(self) -> Result<()> {
        self.set_open_state(false);

        if self.is_connection_closed() {
            return Ok(());
        }

        let responder_rx = self.register_responder(CloseChannelOk::header()).await?;

        synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, CloseChannel::default().into_frame()),
            responder_rx,
            Frame::CloseChannelOk,
            Error::ChannelCloseError
        )?;

        Ok(())
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if let Ok(true) =
            self.shared
                .is_open
                .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
        {
            trace!("drop and close channel {}", self.channel_id());

            let channel = self.clone();
            tokio::spawn(async move {
                if let Err(err) = channel.close().await {
                    error!(
                        "error occurred during close channel when drop, cause: {}",
                        err
                    );
                }
            });
        }
    }
}

/// A set of arguments for the declaration.
/// The syntax and semantics of these arguments depends on the server implementation.
#[derive(Debug, Clone)]
pub struct ServerSpecificArguments {
    table: FieldTable,
}

impl ServerSpecificArguments {
    pub fn new() -> Self {
        Self {
            table: FieldTable::new(),
        }
    }

    pub fn insert_str(&mut self, key: String, value: &str) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::S(value.try_into().unwrap()),
        );
    }
    pub fn insert_bool(&mut self, key: String, value: bool) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::t(value.try_into().unwrap()),
        );
    }
    pub fn insert_u8(&mut self, key: String, value: u8) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::B(value.try_into().unwrap()),
        );
    }
    pub fn insert_u16(&mut self, key: String, value: u8) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::u(value.try_into().unwrap()),
        );
    }
    pub fn insert_u32(&mut self, key: String, value: u32) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::i(value.try_into().unwrap()),
        );
    }
    pub fn insert_i64(&mut self, key: String, value: i64) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::l(value.try_into().unwrap()),
        );
    }
    // the field table type should be hidden from API
    pub(crate) fn into_field_table(self) -> FieldTable {
        self.table
    }
}
/////////////////////////////////////////////////////////////////////////////
mod basic;
mod exchange;
mod queue;

pub use basic::*;
pub use exchange::*;
pub use queue::*;

use super::callbacks::ChannelCallback;
