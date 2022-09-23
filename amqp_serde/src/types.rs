use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

pub type Bit = u8; //TODO: continuous bits packed in octect
pub type Octect = u8;
pub type Boolean = Octect; // 0 = FALSE, else TRUE
pub type ShortShortUint = u8;
pub type ShortShortInt = i8;
pub type ShortUint = u16;
pub type ShortInt = i16;
pub type LongUint = u32;
pub type LongInt = i32;
pub type LongLongUint = u64;
pub type LongLongInt = i64;
pub type TimeStamp = u64;
pub type Float = f32;
pub type Double = f64;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ShortStr(u8, String);
impl ShortStr {
    pub fn new(s: String) -> Self {
        let len = u8::try_from(s.len()).expect("length out of range for ShortStr");
        Self(len, s)
    }
    pub fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LongStr(u32, String);
impl LongStr {
    pub fn new(s: String) -> Self {
        let len = u32::try_from(s.len()).expect("length out of range for LongStr");
        Self(len, s)
    }
    pub fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

// Follow Rabbit definitions below
// Ref: // https://www.rabbitmq.com/amqp-0-9-1-errata.html#section_3
//----------------------------------------------------------------------------
// 0-9   0-9-1   Qpid/Rabbit  Type               Remarks
// ---------------------------------------------------------------------------
//         t       t            Boolean
//         b       b            Signed 8-bit
//         B       B            Unsigned 8-bit
//         U       s            Signed 16-bit      (A1)
//         u       u            Unsigned 16-bit
//   I     I       I            Signed 32-bit
//         i       i            Unsigned 32-bit
//         L       l            Signed 64-bit      (B)
//         l                    Unsigned 64-bit
//         f       f            32-bit float
//         d       d            64-bit float
//   D     D       D            Decimal
//         s                    Short string       (A2)
//   S     S       S            Long string
//         A       A            Array              (C)
//   T     T       T            Timestamp (u64)
//   F     F       F            Nested Table
//   V     V       V            Void
//                 x            Byte array         (D)

#[derive(Serialize, Deserialize, PartialEq, Debug)]
// binary to dec  e.g. (11.11)₂ = (1 × 2¹) + (1 × 2⁰) + (1 × 2⁻¹) + (1 × 2⁻²) = (3.75)₁₀
pub struct DecimalValue(Octect, LongUint);
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ByteArray(LongUint, Vec<u8>);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[allow(non_camel_case_types)]
pub enum FieldValue {
    t(Boolean),
    b(ShortShortInt),
    B(ShortShortUint),
    // U(ShortInt),     // not exist in RabbitMQ
    s(ShortInt), // used in RabbitMQ equivalent to 'U' in 0-9-1 spec
    u(ShortUint),
    I(LongInt),
    i(LongUint),
    // L(LongLongInt),  // not exist in RabbitMQ
    l(LongLongInt), // RabbitMQ is signed, 0-9-1 spec is unsigned
    f(Float),
    d(Double),
    D(DecimalValue),
    // s(ShortStr),     // not exist in RabbitMQ
    S(LongStr),
    A(FieldArray),
    T(TimeStamp),
    F(FieldTable),
    V,
    x(ByteArray), // RabbitMQ only
}
pub type FieldName = ShortStr;
pub type FieldTable = HashMap<FieldName, FieldValue>;

// RabbitMQ use LongUint, 0-9-1 spec use LongInt
pub type FieldArray = (LongUint, Vec<FieldValue>);

//////////////////////////////////////////////////////////////////////////////
// AMQP domains mapping to types
pub type PeerProperties = FieldTable;
pub type SecurityToken = LongStr;
