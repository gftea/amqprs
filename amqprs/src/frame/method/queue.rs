use amqp_serde::types::{
    AmqpExchangeName, AmqpMessageCount, AmqpQueueName, Boolean, FieldTable, LongUint, Octect,
    ShortStr, ShortUint,
};
use serde::{Deserialize, Serialize};



mod bit_flag {
    pub mod declare {
        // continous bits packed into one or more octets, starting from the low bit in each octet.
        pub const NO_WAIT: u8 = 0b0001_0000;
        pub const AUTO_DELETE: u8 = 0b0000_1000;
        pub const EXCLUSIVE: u8 = 0b0000_0100;
        pub const DURABLE: u8 = 0b0000_0010;
        pub const PASSIVE: u8 = 0b0000_0001;
    }
    pub mod delete {

        pub const IF_UNUSED: u8 = 0b0000_0001;
        pub const IF_EMPTY: u8 = 0b0000_0010;
        pub const NO_WAIT: u8 = 0b0000_0100;
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DeclareQueue {
    pub ticket: ShortUint,
    pub queue: AmqpQueueName,
    bits: Octect,
    pub arguments: FieldTable,
}

impl DeclareQueue {
    /// set passive to `true`
    pub fn set_passive(&mut self) {
        self.bits |= bit_flag::declare::PASSIVE;
    }
    /// set passive to `false`
    pub fn clear_passive(&mut self) {
        self.bits &= !bit_flag::declare::PASSIVE;
    }
    pub fn set_durable(&mut self) {
        self.bits |= bit_flag::declare::DURABLE;
    }
    pub fn clear_durable(&mut self) {
        self.bits &= !bit_flag::declare::DURABLE;
    }
    pub fn set_auto_delete(&mut self) {
        self.bits |= bit_flag::declare::AUTO_DELETE;
    }
    pub fn clear_auto_delete(&mut self) {
        self.bits &= !bit_flag::declare::AUTO_DELETE;
    }
    pub fn set_exclusive(&mut self) {
        self.bits |= bit_flag::declare::EXCLUSIVE;
    }
    pub fn clear_exclusive(&mut self) {
        self.bits &= !bit_flag::declare::EXCLUSIVE;
    }
    pub fn set_no_wait(&mut self) {
        self.bits |= bit_flag::declare::NO_WAIT;
    }
    pub fn clear_no_wait(&mut self) {
        self.bits &= !bit_flag::declare::NO_WAIT;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeclareQueueOk {
    queue: AmqpQueueName,
    message_count: AmqpMessageCount,
    consumer_count: LongUint,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BindQueue {
    ticket: ShortUint,
    queue: AmqpQueueName,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
    nowait: Boolean,
    arguments: FieldTable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BindQueueOk;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UnbindQueue {
    ticket: ShortUint,
    queue: AmqpQueueName,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
    arguments: FieldTable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnbindQueueOk;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DeleteQueue {
    ticket: ShortUint,
    queue: AmqpQueueName,
    bits: Octect,
}
impl DeleteQueue {
    pub fn set_if_unused(&mut self) {
        self.bits |= bit_flag::delete::IF_UNUSED;
    }
    pub fn clear_if_unused(&mut self) {
        self.bits &= !bit_flag::delete::IF_UNUSED;
    }
    pub fn set_if_empty(&mut self) {
        self.bits |= bit_flag::delete::IF_EMPTY;
    }
    pub fn clear_if_empty(&mut self) {
        self.bits &= !bit_flag::delete::IF_EMPTY;
    }
    pub fn set_no_wait(&mut self) {
        self.bits |= bit_flag::delete::NO_WAIT;
    }
    pub fn clear_no_wait(&mut self) {
        self.bits &= !bit_flag::delete::NO_WAIT;
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteQueueOk;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PurgeQueue {
    ticket: ShortUint,
    queue: AmqpQueueName,
    nowait: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurgeQueueOk {
    message_count: AmqpMessageCount,
}
