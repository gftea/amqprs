use amqp_serde::types::{
    AmqpExchangeName, AmqpMessageCount, AmqpQueueName, Boolean, FieldTable, LongLongUint, LongUint,
    Octect, ShortStr, ShortUint,
};
use serde::{Deserialize, Serialize};

mod bit_flag {
    pub mod consume {
        use amqp_serde::types::Octect;
        pub const NO_LOCAL: Octect = 0b0000_0001;
        pub const NO_ACK: Octect = 0b0000_0010;
        pub const EXCLUSIVE: Octect = 0b0000_0100;
        pub const NO_WAIT: Octect = 0b0000_1000;
    }
    pub mod publish {
        use amqp_serde::types::Octect;
        pub const MANDATORY: Octect = 0b0000_0001;
        pub const IMMEDIATE: Octect = 0b0000_0010;
    }
    pub mod nack {
        use amqp_serde::types::Octect;
        pub const MULTIPLE: Octect = 0b0000_0001;
        pub const REQUEUE: Octect = 0b0000_0010;
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Qos {
    pub prefetch_size: LongUint,
    pub prefetch_count: ShortUint,
    pub global: Boolean,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct QosOk;

#[derive(Debug, Serialize, Deserialize)]
pub struct Consume {
    pub ticket: ShortUint,
    pub queue: AmqpQueueName,
    pub consumer_tag: ShortStr,
    pub bits: Octect,
    pub arguments: FieldTable,
}
impl Consume {
    pub fn set_no_local(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::consume::NO_LOCAL;
        } else {
            self.bits &= !bit_flag::consume::NO_LOCAL;
        }
    }
    pub fn set_no_ack(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::consume::NO_ACK;
        } else {
            self.bits &= !bit_flag::consume::NO_ACK;
        }
    }
    pub fn set_exclusive(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::consume::EXCLUSIVE;
        } else {
            self.bits &= !bit_flag::consume::EXCLUSIVE;
        }
    }
    pub fn set_nowait(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::consume::NO_WAIT;
        } else {
            self.bits &= !bit_flag::consume::NO_WAIT;
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumeOk {
    pub consumer_tag: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cancel {
    pub consumer_tag: ShortStr,
    pub no_wait: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelOk {
    pub consumer_tag: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Publish {
    pub ticket: ShortUint,
    pub exchange: AmqpExchangeName,
    pub routing_key: ShortStr,
    pub bits: Octect,
}
impl Publish {
    pub fn set_mandatory(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::publish::MANDATORY;
        } else {
            self.bits &= !bit_flag::publish::MANDATORY;
        }
    }
    pub fn set_immediate(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::publish::IMMEDIATE;
        } else {
            self.bits &= !bit_flag::publish::IMMEDIATE;
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Return {
    pub reply_code: ShortUint,
    pub reply_text: ShortStr,
    pub exchange: AmqpExchangeName,
    pub routing_key: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deliver {
    consumer_tag: ShortStr,
    delivery_tag: LongLongUint,
    redelivered: Boolean,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
}

impl Deliver {
    pub fn consumer_tag(&self) -> &String {
        &self.consumer_tag
    }
    pub fn delivery_tag(&self) -> u64 {
        self.delivery_tag
    }

    pub fn redelivered(&self) -> bool {
        self.redelivered
    }

    pub fn exchange(&self) -> &String {
        &self.exchange
    }

    pub fn routing_key(&self) -> &String {
        &self.routing_key
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Get {
    pub ticket: ShortUint,
    pub queue: AmqpQueueName,
    pub no_ack: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOk {
    pub delivery_tag: LongLongUint,
    pub redelivered: Boolean,
    pub exchange: AmqpExchangeName,
    pub routing_key: ShortStr,
    pub message_count: AmqpMessageCount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetEmpty {
    pub cluster_id: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ack {
    pub delivery_tag: LongLongUint,
    pub mutiple: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reject {
    pub delivery_tag: LongLongUint,
    pub requeue: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecoverAsync {
    pub requeue: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recover {
    pub requeue: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecoverOk;

#[derive(Debug, Serialize, Deserialize)]
pub struct Nack {
    pub delivery_tag: LongLongUint,
    pub bits: Octect,
}
impl Nack {
    pub fn set_multiple(&mut self) {
        self.bits |= bit_flag::nack::MULTIPLE;
    }
    pub fn clear_multiple(&mut self) {
        self.bits &= !bit_flag::nack::MULTIPLE;
    }
    pub fn set_requeue(&mut self) {
        self.bits |= bit_flag::nack::REQUEUE;
    }
    pub fn clear_requeue(&mut self) {
        self.bits &= !bit_flag::nack::REQUEUE;
    }
}
