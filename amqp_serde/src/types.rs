use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use std::convert::TryFrom;
// Elementary domains
//    <!-- Elementary domains -->
//   <domain name = "bit"        type = "bit"       label = "single bit" />
//   <domain name = "octet"      type = "octet"     label = "single octet" />
//   <domain name = "short"      type = "short"     label = "16-bit integer" />
//   <domain name = "long"       type = "long"      label = "32-bit integer" />
//   <domain name = "longlong"   type = "longlong"  label = "64-bit integer" />
//   <domain name = "shortstr"   type = "shortstr"  label = "short string (max. 256 characters)" />
//   <domain name = "longstr"    type = "longstr"   label = "long string" />
//   <domain name = "timestamp"  type = "timestamp" label = "64-bit timestamp" />
//   <domain name = "table"      type = "table"     label = "field table" />

pub type Bit = u8; //TODO: implement or use crate https://docs.rs/bitflags/latest/bitflags/
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
        let len = u8::try_from(s.len()).expect("length out of range");
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
        let len = u32::try_from(s.len()).expect("length out of range");
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
pub struct DecimalValue(Octect, LongUint);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[allow(non_camel_case_types)]
pub enum FieldValue {
    t(Boolean),
    b(ShortShortInt),
    B(ShortShortUint),
    // U(ShortInt), // not exist in RabbitMQ
    s(ShortInt), // used in RabbitMQ equivalent to 'U' in 0-9-1 spec
    u(ShortUint),
    I(LongInt),
    i(LongUint),
    // L(LongLongInt), // not exist in RabbitMQ
    l(LongLongInt),  // RabbitMQ is signed, 0-9-1 spec is unsigned
    f(Float),
    d(Double),
    D(DecimalValue),
    // s(ShortStr),  // not exist in RabbitMQ
    S(LongStr),
    A(FieldArray),
    T(TimeStamp),
    F(FieldTable),
    V,
    x(LongUint, Vec<u8>), // RabbitMQ only
}
pub type FieldName = ShortStr;
pub type FieldTable = HashMap<FieldName, FieldValue>;

// RabbitMQ use LongUint, 0-9-1 spec use LongInt
pub type FieldArray = (LongUint, Vec<FieldValue>);

// AMQP domains
pub type PeerProperties = FieldTable;
