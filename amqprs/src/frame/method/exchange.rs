use amqp_serde::types::{AmqpExchangeName, FieldTable, Octect, ShortStr, ShortUint, Boolean};
use serde::{Deserialize, Serialize};

mod bit_flag {
    pub mod declare {
        // continous bits packed into one or more octets, starting from the low bit in each octet.
        pub const NO_WAIT: u8 = 0b0001_0000;
        pub const INTERNAL: u8 = 0b0000_1000;
        pub const AUTO_DELETE: u8 = 0b0000_0100;
        pub const DURABLE: u8 = 0b0000_0010;
        pub const PASSIVE: u8 = 0b0000_0001;
    }
    pub mod delete {

        pub const IF_UNUSED: u8 = 0b0000_0001;
        pub const NO_WAIT: u8 = 0b0000_0010;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Declare {
    pub ticket: ShortUint,
    pub exchange: AmqpExchangeName,
    pub typ: ShortStr,
    bits: Octect,
    pub arguments: FieldTable,
}

impl Default for Declare {
    fn default() -> Self {
        Self {
            ticket: 0,
            exchange: "amq.direct".try_into().unwrap(),
            typ: "direct".try_into().unwrap(),
            bits: 0b0000_0000,
            arguments: FieldTable::new(),
        }
    }
}
impl Declare {
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
    pub fn set_internal(&mut self) {
        self.bits |= bit_flag::declare::INTERNAL;
    }
    pub fn clear_internal(&mut self) {
        self.bits &= !bit_flag::declare::INTERNAL;
    }
    pub fn set_no_wait(&mut self) {
        self.bits |= bit_flag::declare::NO_WAIT;
    }
    pub fn clear_no_wait(&mut self) {
        self.bits &= !bit_flag::declare::NO_WAIT;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeclareOk;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Delete {
    ticket: ShortUint,
    exchange: AmqpExchangeName,
    bits: Octect,
}
impl Delete {
    pub fn set_if_unused(&mut self) {
        self.bits |= bit_flag::delete::IF_UNUSED;
    }
    pub fn clear_if_unused(&mut self) {
        self.bits &= !bit_flag::delete::IF_UNUSED;
    }
    pub fn set_no_wait(&mut self) {
        self.bits |= bit_flag::delete::NO_WAIT;
    }
    pub fn clear_no_wait(&mut self) {
        self.bits &= !bit_flag::delete::NO_WAIT;
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteOk;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Bind{
    ticket: ShortUint,
    destination: AmqpExchangeName,
    source: AmqpExchangeName,
    routing_key: ShortStr,
    nowait: Boolean,
    arguments: FieldTable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BindOk;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Unbind{
    ticket: ShortUint,
    destination: AmqpExchangeName,
    source: AmqpExchangeName,
    routing_key: ShortStr,
    nowait: Boolean,
    arguments: FieldTable,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct UnbindOk;