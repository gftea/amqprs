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
    ticket: ShortUint,
    exchange: ShortStr,
    typ: ShortStr,
    bits: Octect,
    arguments: FieldTable,
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
    pub fn new(
        passive: bool,
        durable: bool,
        auto_delete: bool,
        internal: bool,
        no_wait: bool,
    ) -> Self {
        let mut bits = 0b0000_0000;
        if passive {
            bits |= PASSIVE;
        }
        if durable {
            bits |= DURABLE;
        }
        if auto_delete {
            bits |= AUTO_DELETE;            
        }
        if internal {
            bits |= INTERNAL;
        }
        if no_wait {
            bits |= NO_WAIT;
        }
        Declare {
            ticket: 0,
            exchange: "amq.direct".try_into().unwrap(),
            typ: "direct".try_into().unwrap(),
            bits,
            arguments: FieldTable::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeclareOk;
