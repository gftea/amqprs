use std::{collections::BTreeMap, sync::Arc};

use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    RwLock,
};


use super::{BufferReader, BufferWriter, Response, SplitConnection};
const CHANNEL_BUFFER_SIZE: usize = 8;

struct ChannelIdRepository {
    channel_max: ShortUint,
    watermark: AmqpChannelId,
    freepool: Vec<AmqpChannelId>,
}
impl ChannelIdRepository {
    fn new(channel_max: ShortUint) -> Self {
        Self {
            channel_max,
            watermark: 0,
            freepool: vec![],
        }
    }
    fn allocate(&mut self) -> AmqpChannelId {
        assert!(self.watermark < self.channel_max);
        // reuse id from freepool if any
        if self.freepool.len() > 0 {
            self.freepool.pop().unwrap()
        } else {
            // it should never overflow because max number of channel should not exceed 65535
            // and id is always recycled from closed channel
            self.watermark = self.watermark.checked_add(1).unwrap();
            self.watermark
        }
    }
    fn release(&mut self, id: &AmqpChannelId) {
        if !self.freepool.contains(id) {
            self.freepool.push(*id);
        }
    }
}

pub type MessageHandler = Box<dyn Fn() -> () + Send + Sync>;
pub type MessageHandlerQueue= BTreeMap<String, MessageHandler>;

pub struct ChannelManager {
    id_allocator: Arc<RwLock<ChannelIdRepository>>,
    responders: Arc<RwLock<BTreeMap<AmqpChannelId, Sender<Response>>>>,
    handlers: Arc<RwLock<BTreeMap<AmqpChannelId, MessageHandlerQueue>>>,
}

// AMQP channel manager handle allocation of AMQP channel id and messaging channel
impl ChannelManager {
    pub fn new(channel_max: ShortUint) -> Self {
        Self {
            id_allocator: Arc::new(RwLock::new(ChannelIdRepository::new(channel_max))),
            responders: Arc::new(RwLock::new(BTreeMap::new())),
            handlers: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
    pub async fn alloc(&mut self) -> (AmqpChannelId, Receiver<Response>) {
        let id = self.id_allocator.write().await.allocate();
        self.handlers.write().await.insert(id, BTreeMap::new());
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        match self.responders.write().await.insert(id, tx) {
            Some(_) => panic!("channel already exist"),
            None => (id, rx),
        }
    }

    pub async fn free(&mut self, id: &AmqpChannelId) -> Option<Sender<Response>> {
        self.id_allocator.write().await.release(id);
        self.handlers.write().await.remove(id);
        self.responders.write().await.remove(id)
    }

    pub async fn set_handler(
        &mut self,
        channel_id: AmqpChannelId,
        consumer_tag: String,
        handler: MessageHandler,
    ) {
        let mut guard = self.handlers.write().await;
        let handlers = guard.get_mut(&channel_id).unwrap();
        handlers.insert(consumer_tag, handler);
        
    }

    pub async fn get_handler(&self, channel_id: AmqpChannelId, consumer_tag: String)   {
        let guard = self.handlers.read().await;
        let handlers = guard.get(&channel_id).unwrap();
        handlers.get(&consumer_tag).unwrap()()
    }



}
