use amqp_serde::types::{FieldTable, Octect, ShortStr, ShortUint};

const NO_WAIT: u8 = 0b0000_0001;
const INTERNAL: u8 = 0b0000_0010;
const AUTO_DELETE: u8 = 0b0000_0100;
const DURABLE: u8 = 0b0000_1000;
const PASSIVE: u8 = 0b0001_0000;

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
            exchange: "".try_into().unwrap(),
            typ: "direct".try_into().unwrap(),
            bits: 0b0000_0000,
            arguments: FieldTable::new(),
        }
    }
}
pub struct DeclareOk {}
