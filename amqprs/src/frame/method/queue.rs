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
    ticket: ShortUint,
    queue: AmqpQueueName,
    bits: Octect,
    arguments: FieldTable,
}

impl DeclareQueue {
    pub fn new(
        ticket: ShortUint,
        queue: AmqpQueueName,
        bits: Octect,
        arguments: FieldTable,
    ) -> Self {
        Self {
            ticket,
            queue,
            bits,
            arguments,
        }
    }

    /// set passive to `true`
    pub fn set_passive(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::declare::PASSIVE;
        } else {
            self.bits &= !bit_flag::declare::PASSIVE;
        }
    }
    pub fn set_durable(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::declare::DURABLE;
        } else {
            self.bits &= !bit_flag::declare::DURABLE;
        }
    }
    pub fn set_auto_delete(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::declare::AUTO_DELETE;
        } else {
            self.bits &= !bit_flag::declare::AUTO_DELETE;
        }
    }
    pub fn set_exclusive(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::declare::EXCLUSIVE;
        } else {
            self.bits &= !bit_flag::declare::EXCLUSIVE;
        }
    }
    pub fn set_no_wait(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::declare::NO_WAIT;
        } else {
            self.bits &= !bit_flag::declare::NO_WAIT;
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeclareQueueOk {
    queue: AmqpQueueName,
    message_count: AmqpMessageCount,
    consumer_count: LongUint,
}

impl DeclareQueueOk {
    pub fn new(
        queue: AmqpQueueName,
        message_count: AmqpMessageCount,
        consumer_count: LongUint,
    ) -> Self {
        Self {
            queue,
            message_count,
            consumer_count,
        }
    }

    pub fn queue(&self) -> &String {
        &self.queue
    }

    pub fn message_count(&self) -> u32 {
        self.message_count
    }

    pub fn consumer_count(&self) -> u32 {
        self.consumer_count
    }
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

impl BindQueue {
    pub fn new(
        ticket: ShortUint,
        queue: AmqpQueueName,
        exchange: AmqpExchangeName,
        routing_key: ShortStr,
        nowait: Boolean,
        arguments: FieldTable,
    ) -> Self {
        Self {
            ticket,
            queue,
            exchange,
            routing_key,
            nowait,
            arguments,
        }
    }
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

impl UnbindQueue {
    pub fn new(
        ticket: ShortUint,
        queue: AmqpQueueName,
        exchange: AmqpExchangeName,
        routing_key: ShortStr,
        arguments: FieldTable,
    ) -> Self {
        Self {
            ticket,
            queue,
            exchange,
            routing_key,
            arguments,
        }
    }
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
    pub fn new(ticket: ShortUint, queue: AmqpQueueName, bits: Octect) -> Self {
        Self {
            ticket,
            queue,
            bits,
        }
    }

    pub fn set_if_unused(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::delete::IF_UNUSED;
        } else {
            self.bits &= !bit_flag::delete::IF_UNUSED;
        }
    }
    pub fn set_if_empty(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::delete::IF_EMPTY;
        } else {
            self.bits &= !bit_flag::delete::IF_EMPTY;
        }
    }
    pub fn set_no_wait(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::delete::NO_WAIT;
        } else {
            self.bits &= !bit_flag::delete::NO_WAIT;
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteQueueOk {
    message_count: AmqpMessageCount,
}

impl DeleteQueueOk {
    pub fn message_count(&self) -> u32 {
        self.message_count
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PurgeQueue {
    ticket: ShortUint,
    queue: AmqpQueueName,
    nowait: Boolean,
}

impl PurgeQueue {
    pub fn new(ticket: ShortUint, queue: AmqpQueueName, nowait: Boolean) -> Self {
        Self {
            ticket,
            queue,
            nowait,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurgeQueueOk {
    message_count: AmqpMessageCount,
}

impl PurgeQueueOk {
    pub fn message_count(&self) -> u32 {
        self.message_count
    }
}
