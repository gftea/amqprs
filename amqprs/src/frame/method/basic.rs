use amqp_serde::types::{LongUint, ShortUint, Boolean, AmqpQueueName, FieldTable, AmqpExchangeName, Octect, ShortStr, LongLongUint, AmqpMessageCount};
use serde::{Serialize, Deserialize};

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
    bits: Octect,
    pub arguments: FieldTable,
}
impl Consume {
    pub fn set_no_local(&mut self) {
        self.bits |= bit_flag::consume::NO_LOCAL;
    }
    pub fn clear_no_local(&mut self) {
        self.bits &= !bit_flag::consume::NO_LOCAL;
    }
    pub fn set_no_ack(&mut self) {
        self.bits |= bit_flag::consume::NO_ACK;
    }
    pub fn clear_no_ack(&mut self) {
        self.bits &= !bit_flag::consume::NO_ACK;
    }
    pub fn set_exclusive(&mut self) {
        self.bits |= bit_flag::consume::EXCLUSIVE;
    }
    pub fn clear_exclusive(&mut self) {
        self.bits &= !bit_flag::consume::EXCLUSIVE;
    }
    pub fn set_nowait(&mut self) {
        self.bits |= bit_flag::consume::NO_WAIT;
    }
    pub fn clear_nowait(&mut self) {
        self.bits &= !bit_flag::consume::NO_WAIT;
    }            
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumeOk {
    consumer_tag: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cancel {
    consumer_tag: ShortStr,
    no_wait: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelOk {
    consumer_tag: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Publish {
    ticket: ShortUint,
    exchange: AmqpExchangeName,
    routing_key : ShortStr,
    bits: Octect,
}
impl Publish {
    pub fn set_mandatory(&mut self) {
        self.bits |= bit_flag::publish::MANDATORY;
    }
    pub fn clear_mandatory(&mut self) {
        self.bits &= !bit_flag::publish::MANDATORY;
    }
    pub fn set_immediate(&mut self) {
        self.bits |= bit_flag::publish::IMMEDIATE;
    }
    pub fn clear_immediate(&mut self) {
        self.bits &= !bit_flag::publish::IMMEDIATE;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Return {
    reply_code: ShortUint,
    reply_text: ShortStr,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deliver {
    consumer_tag: ShortStr,
    delivery_tag: LongLongUint,
    redelivered: Boolean,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Get {
    ticket: ShortUint,
    queue: AmqpQueueName,
    no_ack: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOk {
    delivery_tag: LongLongUint,
    redelivered: Boolean,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
    message_count: AmqpMessageCount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetEmpty {
    cluster_id: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ack {
    delivery_tag: LongLongUint,
    mutiple: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reject {
    delivery_tag: LongLongUint,
    requeue: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct  RecoverAsync {
    requeue: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct  Recover {
    requeue: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct  RecoverOk;

#[derive(Debug, Serialize, Deserialize)]
pub struct  Nack {
    delivery_tag: LongLongUint,
    bits: Octect
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