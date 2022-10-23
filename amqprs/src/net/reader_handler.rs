use std::{collections::BTreeMap, sync::Arc};

use amqp_serde::types::{AmqpChannelId, AmqpConsumerTag, AmqpReplyCode, AmqpReplyText, ShortUint, ShortStr};
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver, Sender},
    oneshot, RwLock,
};

use crate::{
    api::channel,
    frame::{
        BasicPropertities, Close, CloseChannel, CloseChannelOk, CloseOk, Frame, CONN_CTRL_CHANNEL,
    },
};

use super::{BufReader, BufWriter, Error, SplitConnection};
const CHANNEL_BUFFER_SIZE: usize = 8;

/////////////////////////////////////////////////////////////////////////////
pub type OutgoingMessage = (AmqpChannelId, Frame);

// TODO: move definition to receiver side, a.k.a API layer
#[derive(Debug)]
pub enum IncomingMessage {
    Ok(Frame),
    Exception(AmqpReplyCode, String),
}

struct ConsumerMessage;

pub struct RegisterResponder {
    channel_id: AmqpChannelId,
    responder: Sender<IncomingMessage>,
    acker: oneshot::Sender<()>,
}
pub struct RegisterConsumer {
    channel_id: AmqpChannelId,
    consumer: Sender<ConsumerMessage>,
    acker: oneshot::Sender<()>,
}
pub enum RegistrationCommand {
    Responder(RegisterResponder),
    Consumer(RegisterConsumer),
}
/////////////////////////////////////////////////////////////////////////////

pub struct ReaderHandler {
    stream: BufReader,
    // /// sender half to forward received server response to connection
    // connection_response_tx: Sender<Response>,
    /// sender half to forward message to `WriterHandler`
    forwarder: Sender<OutgoingMessage>,

    /// receiver half to receive registeration message from AMQ Channel
    register: Receiver<RegistrationCommand>,

    /// sender half to forward message to AMQ Connection/Channel
    responders: BTreeMap<AmqpChannelId, Sender<IncomingMessage>>,

    /// registery of sender half of channel consumers
    consumers: BTreeMap<AmqpChannelId, BTreeMap<AmqpConsumerTag, Sender<ConsumerMessage>>>,

    /// Notify WriterHandler to shutdown.
    /// If reader handler exit first, it will notify writer handler to shutdown.
    /// If writer handler exit first, socket connection will be shutdown because the writer half drop,
    /// so socket read will return, and reader handler can detect connection shutdown without separate signal.
    #[allow(dead_code /* notify shutdown just by dropping the instance */)]
    shutdown_notifier: broadcast::Sender<()>,
    // channel_manager: Arc<RwLock<ChannelManager>>,
}

impl ReaderHandler {
    async fn handle_close(&self, close: &Close) -> Result<(), Error> {
        println!("respond CloseOk to server");
        self.forwarder
            .send((CONN_CTRL_CHANNEL, CloseOk::default().into_frame()))
            .await?;

        Ok(())
    }
    async fn handle_close_ok(&self, channel_id: AmqpChannelId, frame: Frame) -> Result<(), Error> {
        println!("got CloseOk");
        self.responders
            .get(&channel_id)
            .ok_or_else(|| Error::InternalChannelError(format!("responder not found for CloseOk")))?
            .send(IncomingMessage::Ok(frame))
            .await?;
        Ok(())
    }

    async fn handle_close_channel(
        &mut self,
        channel_id: AmqpChannelId,
        close_channel: &CloseChannel,
    ) -> Result<(), Error> {
        println!("respond CloseChannelOk to server, channel id: {channel_id} ");
        self.forwarder
            .send((channel_id, CloseChannelOk::default().into_frame()))
            .await?;

        let responder = self.responders.remove(&channel_id).ok_or_else(|| {
            Error::InternalChannelError(format!("responder not found for: {:?}", close_channel))
        })?;

        // respond as Exception message
        responder
            .send(IncomingMessage::Exception(
                close_channel.reply_code,
                close_channel.reply_text.clone().into(),
            ))
            .await?;
        Ok(())
    }
    async fn handle_close_channel_ok(
        &mut self,
        channel_id: AmqpChannelId,
        frame: Frame,
    ) -> Result<(), Error> {
        println!("got CloseChannelOk, channel id: {channel_id} ");
        let responder = self.responders.remove(&channel_id).ok_or_else(|| {
            Error::InternalChannelError(format!("responder not found for CloseChannelOk"))
        })?;

        responder.send(IncomingMessage::Ok(frame)).await?;
        Ok(())
    }

    /// If OK, user can continue to handle frame
    /// If NOK, user should stop consuming frame 
    /// TODO: implement as Iterator, then user do not need to care about the error
    async fn handle_frame(&mut self, channel_id: AmqpChannelId, frame: Frame) -> Result<(), Error> {
        
        match &frame {
            // TODO: handle Blocked and Unblocked from server
            Frame::Blocked(..) => todo!(),
            Frame::Unblocked(..) => todo!(),

            // Server request to close connection
            Frame::Close(_, close) => {
                assert_eq!(CONN_CTRL_CHANNEL, channel_id, "must be from channel 0");
                self.handle_close(close).await?;
                // always indicate there is error
                Err(Error::AMQPError(format!(
                    "{}: {}, casue: {}:{}",
                    close.reply_code,
                    <ShortStr as Into<String>>::into(close.reply_text.clone()),
                    close.class_id,
                    close.method_id
                )))
            }
            // Close connection response from server
            Frame::CloseOk(_, _) => {
                assert_eq!(CONN_CTRL_CHANNEL, channel_id, "must be from channel 0");
                self.handle_close_ok(channel_id, frame).await?;
                // always indicate it is a normal shutdown
                Err(Error::PeerShutdown)
            }
            // Server request to close channel
            Frame::CloseChannel(_, close_channel) => {
                self.handle_close_channel(channel_id, close_channel).await
            }
            // Close channel response from server
            Frame::CloseChannelOk(..) => {
                self.handle_close_channel_ok(channel_id, frame).await
            }
            Frame::HeartBeat(_) => {
                println!("handle heartbeat...");
                Ok(())
            }
            _ => {
                // respond to synchronous request
                let responder = self.responders.get(&channel_id).ok_or_else(|| {
                    Error::InternalChannelError(format!("responder not found for: {:?}", frame))
                })?;
                responder.send(IncomingMessage::Ok(frame)).await?;

                Ok(())
            }
        }
    }

    async fn run_until_shutdown(mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.register.recv() => {

                }
                Ok((channel_id, frame)) = self.stream.read_frame() => {
                    if let Err(err) = self.handle_frame(channel_id, frame).await {
                        println!("{}", err);
                        break;
                    }

                }
                else => {
                    break;
                }
            }
        }

        // `self` will drop, so the `self.shutdown_notifier`
        // all tasks which have `subscribed` to `shutdown_notifier` will be notified
        println!("shutdown reader handler!");
    }
}

pub struct WriterHandler {
    stream: BufWriter,
    /// receiver half to receive messages from connection and channels
    request_rx: Receiver<OutgoingMessage>,
    /// listen to shutdown signal
    shutdown: broadcast::Receiver<()>,
    channel_manager: Arc<RwLock<ChannelManager>>,
}

impl WriterHandler {
    async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    println!("received shutdown");
                    // TODO: send close to server?
                    break;
                },
                msg = self.request_rx.recv() => {
                    match msg {
                        Some((channel_id, frame)) => {
                            // handle server close request internally
                            match &frame {
                                Frame::CloseOk(..) => {
                                    assert_eq!(CONN_CTRL_CHANNEL, channel_id);
                                    println!("respond CloseOk to server");
                                    self.stream.write_frame(channel_id, frame).await.unwrap();
                                    break;
                                }
                                Frame::CloseChannelOk(..) => {
                                    println!("respond CloseChannelOk to server");
                                }
                                _ => (),
                            }

                            self.stream.write_frame(channel_id, frame).await.unwrap();
                        },
                        None => break,
                    }
                }
            };
        }
        self.channel_manager.write().await.clear();
        println!("shutdown write connection handler!");
    }
}

pub type OnMessageCallback = Box<dyn Fn() -> () + Send + Sync>;
pub struct ChannelManager {
    channel_max: ShortUint,
    last_allocated_id: AmqpChannelId,
    free_ids: Vec<AmqpChannelId>,
    channels: BTreeMap<AmqpChannelId, Sender<IncomingMessage>>,
    callback_queue: BTreeMap<(AmqpChannelId, String), OnMessageCallback>,
}

// AMQP channel manager handle allocation of AMQP channel id and messaging channel
impl ChannelManager {
    pub fn new(channel_max: ShortUint) -> Self {
        // `DEFAULT_CONNECTION_CHANNEL` is reserved for connection level message
        // the channel manager only allocate id above DEFAULT_CONNECTION_CHANNEL
        Self {
            channel_max,
            last_allocated_id: CONN_CTRL_CHANNEL,
            free_ids: vec![],
            channels: BTreeMap::new(),
            callback_queue: BTreeMap::new(),
        }
    }

    pub fn insert_callback(
        &mut self,
        channel_id: AmqpChannelId,
        consumer_tag: String,
        boxed_callback: OnMessageCallback,
    ) {
        self.callback_queue
            .insert((channel_id, consumer_tag), boxed_callback);
    }

    // allocate a channel id or reuse a previous free id
    fn alloc_id(&mut self) -> ShortUint {
        assert!(self.channels.len() < self.channel_max as usize);
        // if any free id from closed channel
        if self.free_ids.len() > 0 {
            self.free_ids.pop().unwrap()
        } else {
            // it should never overflow because max number of channel should not exceed 65535
            // and free id will be recycled
            self.last_allocated_id = self.last_allocated_id.checked_add(1).unwrap();
            self.last_allocated_id
        }
    }

    // check if the channel exists
    fn check(&self, channel_id: &AmqpChannelId) -> bool {
        self.channels.contains_key(channel_id)
    }

    // get sender half to be used for forwarding mesaage to AMQP channel
    fn get(&self, channel_id: &AmqpChannelId) -> Option<&Sender<IncomingMessage>> {
        self.channels.get(channel_id)
    }

    // store sender half associated with the channel id
    // if channel id already exist, it will silently overwrite the associated value
    fn store(&mut self, channel_id: AmqpChannelId, sender: Sender<IncomingMessage>) {
        self.channels.insert(channel_id, sender);
    }

    // remove sender half
    // if exist, return the Sender half, else None
    fn remove(&mut self, channel_id: AmqpChannelId) -> Option<Sender<IncomingMessage>> {
        let sender = self.channels.remove(&channel_id);
        if !self.free_ids.contains(&channel_id) {
            self.free_ids.push(channel_id);
        }
        sender
    }

    fn clear(&mut self) {
        self.channels.clear();
        self.free_ids.clear();
        self.last_allocated_id = CONN_CTRL_CHANNEL;
    }
}

/// This hides the AMQP connection and channel management for the user.
/// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
/// The `tx` send message to `WriterHandler`, and `rx` is to receive message from `ReaderHandler`
/// Requests initiated from server are handled internally, e.g. hearbeat, close request from servers, etc
pub struct ConnectionManager {
    /// The sender half to forward message to `WriterHandler`
    tx: Sender<OutgoingMessage>,
    /// The receiver half to receive message from  `ReaderHandler`
    rx: Receiver<IncomingMessage>,
    /// The channel id allocation and management
    channel_manager: Arc<RwLock<ChannelManager>>,
}

impl ConnectionManager {
    pub async fn send(
        &self,
        value: OutgoingMessage,
    ) -> Result<(), mpsc::error::SendError<(u16, Frame)>> {
        self.tx.send(value).await
    }
    pub async fn recv(&mut self) -> Option<IncomingMessage> {
        self.rx.recv().await
    }
    pub async fn spawn(connection: SplitConnection, channel_max: ShortUint) -> Self {
        // The Connection Manager will Spawn two  tasks for connection
        // - one task for writer handler
        // - one task for reader handler
        let (request_tx, request_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (response_tx, response_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (reader, writer) = connection.into_split();

        let channel_manager = Arc::new(RwLock::new(ChannelManager::new(channel_max)));

        let (notify_shutdown, shutdown) = broadcast::channel::<()>(1);

        // spawn task for write connection handler
        let mut handler = WriterHandler {
            stream: writer,
            channel_manager: channel_manager.clone(),
            request_rx,
            shutdown,
        };
        tokio::spawn(async move {
            handler.run().await;
        });

        // spawn task for read connection hanlder
        let mut handler = ReaderHandler {
            stream: reader,
            connection_response_tx: response_tx,
            forwarder: request_tx.clone(),
            shutdown_notifier: notify_shutdown,
            channel_manager: channel_manager.clone(),
        };
        tokio::spawn(async move {
            handler.run_until_shutdown().await;
        });

        Self {
            tx: request_tx,
            rx: response_rx,
            channel_manager,
        }
    }

    pub async fn allocate_channel(
        &mut self,
    ) -> (
        AmqpChannelId,
        Sender<OutgoingMessage>,
        Receiver<IncomingMessage>,
        Arc<RwLock<ChannelManager>>,
    ) {
        // A channel has
        // - a sender to send message to write connection handler,  created by cloning sender of channel manager
        // - a receiver to receive message from read connection handler, the sender half will be stored in channel manager
        //   and will be retrieved by read connection handler
        //
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let channel_id = self.channel_manager.write().await.alloc_id();
        if self.channel_manager.read().await.check(&channel_id) {
            unreachable!("duplicated channel id: {channel_id}");
        }
        self.channel_manager.write().await.store(channel_id, tx);
        (
            channel_id,
            self.tx.clone(),
            rx,
            self.channel_manager.clone(),
        )
    }
}
