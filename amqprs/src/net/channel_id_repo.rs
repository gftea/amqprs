use std::collections::{BTreeMap, BTreeSet};

use amqp_serde::types::{AmqpChannelId, ShortUint};

use crate::frame::CONN_CTRL_CHANNEL;

pub(super) struct ChannelIdRepository {
    channel_max: ShortUint,
    watermark: AmqpChannelId,
    /// If value is None, then it is not reserved
    freepool: Vec<AmqpChannelId>,
    reservedpool: Vec<AmqpChannelId>,
}
impl ChannelIdRepository {
    pub fn new(channel_max: ShortUint) -> Self {
        Self {
            channel_max,
            watermark: CONN_CTRL_CHANNEL, // reserved for connection
            freepool: vec![],
            reservedpool: vec![],
        }
    }
    pub fn allocate(&mut self) -> AmqpChannelId {
        assert!(
            self.watermark < self.channel_max,
            "Implementation error in channel allocation"
        );

        match self.freepool.pop() {
            Some(v) => v,
            None => {
                // it should never overflow because max number of channel should not exceed 65535
                // and id is always recycled from closed channel
                self.watermark = self.watermark.checked_add(1).unwrap();

                while self.reservedpool.contains(&self.watermark) {
                    self.watermark = self.watermark.checked_add(1).unwrap();
                }
                self.watermark
            }
        }
    }
    pub fn release(&mut self, id: &AmqpChannelId) -> bool {
        assert_ne!(
            &CONN_CTRL_CHANNEL, id,
            "Connection's default channel cannot be released"
        );
        if let Some(i) = self.reservedpool.iter().position(|v| v == id) {
            self.reservedpool.swap_remove(i);
        }
        if !self.freepool.contains(id) {
            self.freepool.push(*id);
            true
        } else {
            false
        }
    }

    pub fn reserve(&mut self, id: &AmqpChannelId) -> bool {
        if id == &CONN_CTRL_CHANNEL {
            true
        } else if let Some(i) = self.freepool.iter().position(|v| v == id) {
            self.freepool.swap_remove(i);
            true
        } else if id > &self.watermark && !self.reservedpool.contains(id) {
            self.reservedpool.push(*id);
            true
        } else {
            false
        }
    }
}
