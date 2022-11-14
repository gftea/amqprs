use std::{collections::HashMap, num::TryFromIntError, ops::Deref};

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

pub type Bit = u8; //TODO: continuous bits packed in octect
pub type Octect = u8;
pub type Boolean = bool; // 0 = FALSE, else TRUE
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ShortStr(u8, String);

impl Default for ShortStr {
    fn default() -> Self {
        Self(0, "".to_string())
    }
}
impl From<ShortStr> for String {
    fn from(s: ShortStr) -> Self {
        s.1
    }
}
impl Deref for ShortStr {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl TryFrom<String> for ShortStr {
    type Error = TryFromIntError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let len = u8::try_from(s.len())?;
        Ok(Self(len, s))
    }
}
impl TryFrom<&str> for ShortStr {
    type Error = TryFromIntError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.to_string().try_into()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LongStr(u32, String);

impl Default for LongStr {
    fn default() -> Self {
        Self(0, "".to_string())
    }
}

impl TryFrom<String> for LongStr {
    type Error = TryFromIntError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let len = u32::try_from(s.len())?;
        Ok(Self(len, s))
    }
}
impl TryFrom<&str> for LongStr {
    type Error = TryFromIntError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.to_string().try_into()
    }
}
impl From<LongStr> for String {
    fn from(s: LongStr) -> Self {
        s.1
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

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
//long-uint * 10^(-scale)
pub struct DecimalValue(Octect, LongUint);

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ByteArray(LongUint, Vec<u8>);
impl TryFrom<Vec<u8>> for ByteArray {
    type Error = TryFromIntError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let len = LongUint::try_from(bytes.len())?;
        Ok(Self(len, bytes))
    }
}
impl From<ByteArray> for Vec<u8> {
    fn from(arr: ByteArray) -> Self {
        arr.1
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
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

// RabbitMQ use LongUint
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct FieldArray(LongUint, Vec<FieldValue>);
impl TryFrom<Vec<FieldValue>> for FieldArray {
    type Error = TryFromIntError;

    fn try_from(values: Vec<FieldValue>) -> Result<Self, Self::Error> {
        let len = LongUint::try_from(values.len())?;
        Ok(Self(len, values))
    }
}
impl From<FieldArray> for Vec<FieldValue> {
    fn from(arr: FieldArray) -> Self {
        arr.1
    }
}

/////////////////////////////////////////////////////////////////////////////
// AMQP domains
pub type AmqpPeerProperties = FieldTable;
pub type AmqpSecurityToken = LongStr;
pub type AmqpPath = ShortStr;
pub type AmqpReplyCode = ShortUint;
pub type AmqpReplyText = ShortStr;
pub type AmqpChannelId = ShortUint;
pub type AmqpExchangeName = ShortStr;
pub type AmqpQueueName = ShortStr;
pub type AmqpMessageCount = LongUint;
pub type AmqpConsumerTag = ShortStr;
pub type AmqpDeliveryTag = LongLongUint;
pub type AmqpDuration = LongLongUint;
