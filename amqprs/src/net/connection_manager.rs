use std::{collections::BTreeMap, sync::Arc};

use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver, Sender},
    RwLock,
};

use crate::frame::{CloseChannelOk, CloseOk, Frame, DEFAULT_CONNECTION_CHANNEL};

use super::{BufferReader, BufferWriter, SplitConnection};
const CHANNEL_BUFFER_SIZE: usize = 8;

pub type Message = (AmqpChannelId, Frame);

pub struct ReaderHandler {
    stream: BufferReader,
    response_tx: Sender<Frame>,
    request_tx: Sender<Message>,
    channel_manager: Arc<RwLock<ChannelManager>>,
}
pub struct WriterHandler {
    stream: BufferWriter,
    request_rx: Receiver<Message>,
    channel_manager: Arc<RwLock<ChannelManager>>,
}

pub struct ChannelManager {
    channel_max: ShortUint,
    last_allocated_id: AmqpChannelId,
    free_ids: Vec<AmqpChannelId>,
    channels: BTreeMap<AmqpChannelId, Sender<Frame>>,
}

impl ChannelManager {
    pub fn new(channel_max: ShortUint) -> Self {
        Self {
            channel_max,
            last_allocated_id: 0,
            free_ids: vec![],
            channels: BTreeMap::new(),
        }
    }
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
    fn check(&self, channel_id: &AmqpChannelId) -> bool {
        self.channels.contains_key(channel_id)
    }

    fn get(&self, channel_id: &AmqpChannelId) -> Option<&Sender<Frame>> {
        self.channels.get(channel_id)
    }

    fn add(&mut self, sender: Sender<Frame>) -> AmqpChannelId {
        let channel_id = self.alloc_id();
        self.channels.insert(channel_id, sender);
        channel_id
    }
    fn delete(&mut self, channel_id: AmqpChannelId) -> Option<Sender<Frame>> {
        let sender = self.channels.remove(&channel_id);
        self.free_ids.push(channel_id);
        sender
    }
    fn clear(&mut self) {
        self.channels.clear();
        self.free_ids.clear();
        self.last_allocated_id = 0;
    }
}

pub struct ConnectionManager {
    pub tx: Sender<Message>,
    pub rx: Receiver<Frame>,
    pub channel_manager: Arc<RwLock<ChannelManager>>,
}

impl ConnectionManager {
    /// Spawn management tasks for connection
    pub async fn spawn(connection: SplitConnection, channel_max: ShortUint) -> Self {
        let (request_tx, request_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (response_tx, response_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (reader, writer) = connection.into_split();

        let channel_manager = Arc::new(RwLock::new(ChannelManager::new(channel_max)));

        let (notify_shutdown, mut shutdown) = broadcast::channel::<()>(1);

        // spawn task for write connection
        let mut handler = WriterHandler {
            stream: writer,
            channel_manager: channel_manager.clone(),
            request_rx,
        };
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        println!("received shutdown");
                        break;
                    },
                    msg = handler.request_rx.recv() => {

                            match msg {
                                Some((channel_id, frame)) => {
                                    // handle server close request internally
                                    match &frame {
                                        Frame::CloseOk(..) => {
                                            assert_eq!(DEFAULT_CONNECTION_CHANNEL, channel_id);
                                            println!("respond close ok to server");
                                            handler.stream.write_frame(channel_id, frame).await.unwrap();
                                            break;
                                        }
                                        Frame::CloseChannelOk(..) => {
                                            println!("respond close channel ok to server");
                                        }
                                        _ => (),
                                    }

                                     handler.stream.write_frame(channel_id, frame).await.unwrap();
                                },
                                None => break,
                            }
                    }
                };
            }
            handler.channel_manager.write().await.clear();
            println!("shutdown write connection handler!");
        });

        // spawn task for read connection
        let mut handler = ReaderHandler {
            stream: reader,
            response_tx,
            request_tx: request_tx.clone(),
            channel_manager: channel_manager.clone(),
        };
        tokio::spawn(async move {
            while let Ok((channel_id, frame)) = handler.stream.read_frame().await {
                // handle close request from server
                match &frame {
                    Frame::Close(..) => {
                        assert_eq!(DEFAULT_CONNECTION_CHANNEL, channel_id);
                        println!("forward close ok to writer");
                        handler
                            .request_tx
                            .send((DEFAULT_CONNECTION_CHANNEL, CloseOk::default().into_frame()))
                            .await
                            .unwrap();
                        break;
                    }
                    Frame::CloseOk(..) => {
                        assert_eq!(DEFAULT_CONNECTION_CHANNEL, channel_id);
                        println!("got close connection ok");
                        handler.response_tx.send(frame).await.unwrap();
                        break;
                    }
                    Frame::CloseChannel(..) => {
                        handler.channel_manager.write().await.delete(channel_id);
                        println!("forward close channel ok to writer, id: {channel_id} ");
                        handler
                            .request_tx
                            .send((channel_id, CloseChannelOk::default().into_frame()))
                            .await
                            .unwrap();
                        continue;
                    }

                    Frame::CloseChannelOk(..) => {
                        println!("got close channel ok, id: {channel_id} ");
                        let tx = handler.channel_manager.write().await.delete(channel_id);
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
                    handler.response_tx.send(frame).await.unwrap();
                } else {
                    if let Some(tx) = handler.channel_manager.read().await.get(&channel_id) {
                        if let Err(_) = tx.send(frame).await {
                            // drop  channel
                            handler.channel_manager.write().await.delete(channel_id);
                        }
                    }
                }
            }

            handler.channel_manager.write().await.clear();
            // When `notify_shutdown` is dropped, all tasks which have `subscribe`d will
            // receive the shutdown signal and can exit
            drop(notify_shutdown);
            println!("shutdown read connection handler!");
        });

        //
        Self {
            tx: request_tx,
            rx: response_rx,
            channel_manager,
        }
    }

    pub async fn allocate_channel(&mut self) -> (AmqpChannelId, Sender<Message>, Receiver<Frame>) {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let id = self.channel_manager.write().await.add(tx);
        (id, self.tx.clone(), rx)
    }
}
