use amqp_serde::types::{FieldTable, Octect, ShortStr, ShortUint};
use serde::{Deserialize, Serialize};

// continous bits packed into one or more octets, starting from the low bit in each octet.
const NO_WAIT: u8 = 0b0001_0000;
const INTERNAL: u8 = 0b0000_1000;
const AUTO_DELETE: u8 = 0b0000_0100;
const DURABLE: u8 = 0b0000_0010;
const PASSIVE: u8 = 0b0000_0001;

use super::impl_mapping;
use crate::frame::{Frame, MethodHeader};

impl_mapping!(Declare, 40, 10);
impl_mapping!(DeclareOk, 40, 11);

#[derive(Debug, Serialize)]
pub struct Declare {
    pub ticket: ShortUint,
    pub exchange: ShortStr,
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
        self.bits |= PASSIVE;
    }
    /// set passive to `false`
    pub fn clear_passive(&mut self) {
        self.bits &= !PASSIVE;
    }
    pub fn set_durable(&mut self) {
        self.bits |= DURABLE;
    }
    pub fn clear_durable(&mut self) {
        self.bits &= !DURABLE;
    }
    pub fn set_auto_delete(&mut self) {
        self.bits |= AUTO_DELETE;
    }
    pub fn clear_auto_delete(&mut self) {
        self.bits &= !AUTO_DELETE;
    }
    pub fn set_internal(&mut self) {
        self.bits |= INTERNAL;
    }
    pub fn clear_internal(&mut self) {
        self.bits &= !INTERNAL;
    }
    pub fn set_no_wait(&mut self) {
        self.bits |= NO_WAIT;
    }
    pub fn clear_no_wait(&mut self) {
        self.bits &= !NO_WAIT;
    }
}

#[derive(Debug, Deserialize)]
pub struct DeclareOk;
