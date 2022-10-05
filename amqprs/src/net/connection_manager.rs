use std::{collections::BTreeMap, sync::Arc};

use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver, Sender},
    RwLock,
};

use crate::frame::{CloseChannelOk, CloseOk, Frame, CTRL_CHANNEL};

use super::{BufferReader, BufferWriter, SplitConnection};
const CHANNEL_BUFFER_SIZE: usize = 8;

pub type Message = (AmqpChannelId, Frame);

pub struct ReaderHandler {
    stream: BufferReader,
    /// sender half to forward received server response to client
    response_tx: Sender<Frame>,
    /// sender half to forward message to `WriterHandler` task
    request_tx: Sender<Message>,
    /// to notify WriterHandler task to shutdown
    /// Socket connection will be shutdown as long as the writer half is shutdown
    /// so reader half do not need to listen for shutdown signal.
    #[allow(dead_code /* notify shutdown just by dropping the instance */)]
    notify_shutdown: broadcast::Sender<()>,
    channel_manager: Arc<RwLock<ChannelManager>>,
}

impl ReaderHandler {
    async fn run(&mut self) {
        while let Ok((channel_id, frame)) = self.stream.read_frame().await {
            // handle close request from server
            match &frame {
                Frame::Close(..) => {
                    assert_eq!(CTRL_CHANNEL, channel_id);
                    println!("forward close ok to writer");
                    self.request_tx
                        .send((CTRL_CHANNEL, CloseOk::default().into_frame()))
                        .await
                        .unwrap();
                    break;
                }
                Frame::CloseOk(..) => {
                    assert_eq!(CTRL_CHANNEL, channel_id);
                    println!("got close connection ok");
                    self.response_tx.send(frame).await.unwrap();
                    break;
                }
                Frame::CloseChannel(..) => {
                    self.channel_manager.write().await.remove(channel_id);
                    println!("forward close channel ok to writer, id: {channel_id} ");
                    self.request_tx
                        .send((channel_id, CloseChannelOk::default().into_frame()))
                        .await
                        .unwrap();
                    continue;
                }

                Frame::CloseChannelOk(..) => {
                    println!("got close channel ok, id: {channel_id} ");
                    let tx = self.channel_manager.write().await.remove(channel_id);
                    if let Some(tx) = tx {
                        if let Err(_) = tx.send(frame).await {
                            println!("channel already closed");
                        }
                    }
                    continue;
                }
                Frame::HeartBeat(_) => {
                    println!("heartbeat ok");
                    continue;
                }
                _ => (),
            }
            // forward response to corresponding channel
            if channel_id == 0 {
                // connection's control channel
                self.response_tx.send(frame).await.unwrap();
            } else {
                if let Some(tx) = self.channel_manager.read().await.get(&channel_id) {
                    if let Err(_) = tx.send(frame).await {
                        // drop  channel
                        self.channel_manager.write().await.remove(channel_id);
                    }
                }
            }
        }

        self.channel_manager.write().await.clear();
        // When `notify_shutdown` is dropped, all tasks which have `subscribe`d will
        // receive the shutdown signal and can exit
        println!("shutdown read connection handler!");
    }
}
pub struct WriterHandler {
    stream: BufferWriter,
    /// receiver half to receive client initiated message
    request_rx: Receiver<Message>,
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
                    break;
                },
                msg = self.request_rx.recv() => {
                    match msg {
                        Some((channel_id, frame)) => {
                            // handle server close request internally
                            match &frame {
                                Frame::CloseOk(..) => {
                                    assert_eq!(CTRL_CHANNEL, channel_id);
                                    println!("respond close ok to server");
                                    self.stream.write_frame(channel_id, frame).await.unwrap();
                                    break;
                                }
                                Frame::CloseChannelOk(..) => {
                                    println!("respond close channel ok to server");
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

pub struct ChannelManager {
    channel_max: ShortUint,
    last_allocated_id: AmqpChannelId,
    free_ids: Vec<AmqpChannelId>,
    channels: BTreeMap<AmqpChannelId, Sender<Frame>>,
}

// AMQP channel manager handle allocation of AMQP channel id and messaging channel
impl ChannelManager {
    pub fn new(channel_max: ShortUint) -> Self {
        // `DEFAULT_CONNECTION_CHANNEL` is reserved for connection level message
        // the channel manager only allocate id above DEFAULT_CONNECTION_CHANNEL
        Self {
            channel_max,
            last_allocated_id: CTRL_CHANNEL,
            free_ids: vec![],
            channels: BTreeMap::new(),
        }
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
    fn get(&self, channel_id: &AmqpChannelId) -> Option<&Sender<Frame>> {
        self.channels.get(channel_id)
    }

    // store sender half associated with the channel id
    // if channel id already exist, it will silently overwrite the associated value 
    fn store(&mut self, channel_id: AmqpChannelId, sender: Sender<Frame>)  {
        self.channels.insert(channel_id, sender);
    }

    // remove sender half
    // if exist, return the Sender half, else None
    fn remove(&mut self, channel_id: AmqpChannelId) -> Option<Sender<Frame>> {
        let sender = self.channels.remove(&channel_id);
        if !self.free_ids.contains(&channel_id) {
            self.free_ids.push(channel_id);
        }
        sender
    }

    fn clear(&mut self) {
        self.channels.clear();
        self.free_ids.clear();
        self.last_allocated_id = CTRL_CHANNEL;
    }
}

/// This hides the AMQP connection and channel management for the user.
/// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
/// The `tx` send message to `WriterHandler`, and `rx` is to receive message from `ReaderHandler`
/// Requests initiated from server are handled internally, e.g. hearbeat, close request from servers, etc
pub struct ConnectionManager {
    /// The sender half to forward message to `WriterHandler`
    tx: Sender<Message>,
    /// The receiver half to receive message from  `ReaderHandler`
    rx: Receiver<Frame>,
    /// The channel id allocation and management
    channel_manager: Arc<RwLock<ChannelManager>>,
}


impl ConnectionManager {
    pub async fn send(&self, value: Message) -> Result<(), mpsc::error::SendError<(u16, Frame)>> {
        self.tx.send(value).await
    }
    pub async fn recv(&mut self) -> Option<Frame>  {
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
            response_tx,
            request_tx: request_tx.clone(),
            notify_shutdown,
            channel_manager: channel_manager.clone(),
        };
        tokio::spawn(async move {
            handler.run().await;
        });

        Self {
            tx: request_tx,
            rx: response_rx,
            channel_manager,
        }
    }

    pub async fn allocate_channel(&mut self) -> (AmqpChannelId, Sender<Message>, Receiver<Frame>) {
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
        (channel_id, self.tx.clone(), rx)
    }
}
