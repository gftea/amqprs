use std::fmt;

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

// TX
#[derive(Debug, Serialize, Deserialize)]
pub struct Qos {
    prefetch_size: LongUint,
    prefetch_count: ShortUint,
    global: Boolean,
}

impl Qos {
    pub fn new(prefetch_size: LongUint, prefetch_count: ShortUint, global: Boolean) -> Self {
        Self {
            prefetch_size,
            prefetch_count,
            global,
        }
    }
}

// RX
#[derive(Debug, Serialize, Deserialize)]
pub struct QosOk;

// TX
#[derive(Debug, Serialize, Deserialize)]
pub struct Consume {
    ticket: ShortUint,
    queue: AmqpQueueName,
    consumer_tag: ShortStr,
    bits: Octect,
    arguments: FieldTable,
}

impl Consume {
    pub fn new(
        ticket: ShortUint,
        queue: AmqpQueueName,
        consumer_tag: ShortStr,
        arguments: FieldTable,
    ) -> Self {
        Self {
            ticket,
            queue,
            consumer_tag,
            bits: 0,
            arguments,
        }
    }

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

// RX
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumeOk {
    pub(crate) consumer_tag: ShortStr,
}

/// Used by channel [`cancel`] callback.
///
/// AMQP method frame [cancel](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.cancel).
///
/// [`cancel`]: callbacks/trait.ChannelCallback.html#tymethod.cancel
// TX + RX
#[derive(Debug, Serialize, Deserialize)]
pub struct Cancel {
    consumer_tag: ShortStr,
    no_wait: Boolean,
}

impl Cancel {
    pub(crate) fn new(consumer_tag: ShortStr, no_wait: Boolean) -> Self {
        Self {
            consumer_tag,
            no_wait,
        }
    }

    pub fn consumer_tag(&self) -> &String {
        self.consumer_tag.as_ref()
    }

    pub fn no_wait(&self) -> bool {
        self.no_wait
    }
}

// TX + RX
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelOk {
    pub(crate) consumer_tag: ShortStr,
}

impl CancelOk {
    pub fn new(consumer_tag: ShortStr) -> Self {
        Self { consumer_tag }
    }
}

// TX
#[derive(Debug, Serialize, Deserialize)]
pub struct Publish {
    ticket: ShortUint,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
    bits: Octect,
}
impl Publish {
    pub fn new(ticket: ShortUint, exchange: AmqpExchangeName, routing_key: ShortStr) -> Self {
        Self {
            ticket,
            exchange,
            routing_key,
            bits: 0,
        }
    }

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

/// Used by channel [`publish_return`] callback.
///
/// AMQP method frame [return](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.return).
///
/// [`publish_return`]: callbacks/trait.ChannelCallback.html#tymethod.publish_return
// RX
#[derive(Debug, Serialize, Deserialize)]
pub struct Return {
    reply_code: ShortUint,
    reply_text: ShortStr,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
}
impl fmt::Display for Return {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "Return published message due to '{}: {}', (exchange = {}, routing_key = {})",
            self.reply_code(),
            self.reply_text(),
            self.exchange(),
            self.routing_key()
        ))
    }
}
impl Return {
    pub fn reply_code(&self) -> u16 {
        self.reply_code
    }

    pub fn reply_text(&self) -> &String {
        self.reply_text.as_ref()
    }

    pub fn exchange(&self) -> &String {
        self.exchange.as_ref()
    }

    pub fn routing_key(&self) -> &String {
        self.routing_key.as_ref()
    }
}
/// Used by consumer [`consume`] callback.
///
/// AMQP method frame [deliver](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.deliver).
///
/// [`consume`]: consumer/trait.AsyncConsumer.html#tymethod.consume
// RX
#[derive(Debug, Serialize, Deserialize)]
pub struct Deliver {
    consumer_tag: ShortStr,
    delivery_tag: LongLongUint,
    redelivered: Boolean,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
}
impl fmt::Display for Deliver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Deliver: consumer_tag = {}, delivery_tag = {}, redelivered = {}, exchange = {}, routing_key = {}",
            self.consumer_tag, self.delivery_tag, self.redelivered, self.exchange, self.routing_key
        ))
    }
}

impl Deliver {
    pub fn consumer_tag(&self) -> &String {
        self.consumer_tag.as_ref()
    }
    pub fn delivery_tag(&self) -> u64 {
        self.delivery_tag
    }

    pub fn redelivered(&self) -> bool {
        self.redelivered
    }

    pub fn exchange(&self) -> &String {
        self.exchange.as_ref()
    }

    pub fn routing_key(&self) -> &String {
        self.routing_key.as_ref()
    }
}

// TX
#[derive(Debug, Serialize, Deserialize)]
pub struct Get {
    ticket: ShortUint,
    queue: AmqpQueueName,
    no_ack: Boolean,
}

impl Get {
    pub fn new(ticket: ShortUint, queue: AmqpQueueName, no_ack: Boolean) -> Self {
        Self {
            ticket,
            queue,
            no_ack,
        }
    }
}
/// Part of [`GetMessage`]
///
/// AMQP method frame [get-ok](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.get-ok).
///
/// [`GetMessage`]: channel/type.GetMessage.html
#[derive(Debug, Serialize, Deserialize)]
pub struct GetOk {
    delivery_tag: LongLongUint,
    redelivered: Boolean,
    exchange: AmqpExchangeName,
    routing_key: ShortStr,
    message_count: AmqpMessageCount,
}
impl fmt::Display for GetOk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("GetOk: delivery_tag = {}, redelivered = {}, exchange = {}, routing_key = {}, message_count = {}",
            self.delivery_tag, self.redelivered, self.exchange, self.routing_key, self.message_count
        ))
    }
}
impl GetOk {
    pub fn delivery_tag(&self) -> u64 {
        self.delivery_tag
    }

    pub fn redelivered(&self) -> bool {
        self.redelivered
    }

    pub fn exchange(&self) -> &String {
        self.exchange.as_ref()
    }

    pub fn routing_key(&self) -> &String {
        self.routing_key.as_ref()
    }

    pub fn message_count(&self) -> u32 {
        self.message_count
    }
}

// RX
#[derive(Debug, Serialize, Deserialize)]
pub struct GetEmpty {
    pub(crate) cluster_id: ShortStr,
}

/// Used by channel [`publish_ack`] callback.
///
/// AMQP method frame [ack](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.ack).
///
/// [`publish_ack`]: callbacks/trait.ChannelCallback.html#tymethod.publish_ack
// TX + RX
#[derive(Debug, Serialize, Deserialize)]
pub struct Ack {
    delivery_tag: LongLongUint,
    mutiple: Boolean,
}

impl Ack {
    pub(crate) fn new(delivery_tag: LongLongUint, mutiple: Boolean) -> Self {
        Self {
            delivery_tag,
            mutiple,
        }
    }

    pub fn delivery_tag(&self) -> u64 {
        self.delivery_tag
    }

    pub fn mutiple(&self) -> bool {
        self.mutiple
    }
}

// TX
#[derive(Debug, Serialize, Deserialize)]
pub struct Reject {
    delivery_tag: LongLongUint,
    requeue: Boolean,
}

impl Reject {
    pub fn new(delivery_tag: LongLongUint, requeue: Boolean) -> Self {
        Self {
            delivery_tag,
            requeue,
        }
    }
}

// TX
// Deprecated
// #[derive(Debug, Serialize, Deserialize)]
// pub struct RecoverAsync {
//     requeue: Boolean,
// }

// impl RecoverAsync {
//     pub fn new(requeue: Boolean) -> Self {
//         Self { requeue }
//     }
// }

// TX
#[derive(Debug, Serialize, Deserialize)]
pub struct Recover {
    requeue: Boolean,
}

impl Recover {
    pub fn new(requeue: Boolean) -> Self {
        Self { requeue }
    }
}

// RX
#[derive(Debug, Serialize, Deserialize)]
pub struct RecoverOk;

/// Used by channel [`publish_nack`] callback.
///
/// AMQP method frame [nack](https://www.rabbitmq.com/amqp-0-9-1-reference.html#basic.nack).
///
/// [`publish_nack`]: callbacks/trait.ChannelCallback.html#tymethod.publish_nack
// TX + RX
#[derive(Debug, Serialize, Deserialize)]
pub struct Nack {
    delivery_tag: LongLongUint,
    bits: Octect,
}
impl Nack {
    pub(crate) fn new(delivery_tag: LongLongUint) -> Self {
        Self {
            delivery_tag,
            bits: 0,
        }
    }

    pub fn set_multiple(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::nack::MULTIPLE;
        } else {
            self.bits &= !bit_flag::nack::MULTIPLE;
        }
    }
    pub fn set_requeue(&mut self, value: bool) {
        if value {
            self.bits |= bit_flag::nack::REQUEUE;
        } else {
            self.bits &= !bit_flag::nack::REQUEUE;
        }
    }

    pub fn delivery_tag(&self) -> u64 {
        self.delivery_tag
    }

    pub fn multiple(&self) -> bool {
        self.bits & bit_flag::nack::MULTIPLE > 0
    }

    pub fn requeue(&self) -> bool {
        self.bits & bit_flag::nack::REQUEUE > 0
    }
}
