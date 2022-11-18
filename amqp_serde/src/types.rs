///! AMQP 0-9-1 types for RabbitMQ
///! https://github.com/rabbitmq/rabbitmq-codegen/blob/main/amqp-rabbitmq-0.9.1.json
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::{self, Debug},
    num::TryFromIntError,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

pub type Bit = u8; // No Rust type to represent single bit, but bits are packed in octect
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

/////////////////////////////////////////////////////////////////////////////
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ShortStr(u8, String);
impl fmt::Display for ShortStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}
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

/////////////////////////////////////////////////////////////////////////////
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LongStr(u32, String);
impl fmt::Display for LongStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}
impl Default for LongStr {
    fn default() -> Self {
        Self(0, "".to_string())
    }
}

impl Deref for LongStr {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
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

/////////////////////////////////////////////////////////////////////////////
/// Specification of the decimal value format in RabbitMQ?
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct DecimalValue(Octect, LongUint);

/////////////////////////////////////////////////////////////////////////////
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

/////////////////////////////////////////////////////////////////////////////
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
// #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
// pub struct FieldTable(HashMap<FieldName, FieldValue>);
// impl Default for FieldTable {
//     fn default() -> Self {
//         Self(HashMap::new())
//     }
// }

// impl Deref for FieldTable {
//     type Target = HashMap<FieldName, FieldValue>;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl FieldTable {
//     pub fn new() -> Self {
//         Self(HashMap::new())
//     }
    
//     pub fn insert<V>(&mut self, k: String, v: V) 
//     where
//         V: Any,
//     {
//         let k: FieldName = k.try_into().unwrap();
//         let v = &v as &dyn Any;

//         if v.is::<bool>() {
//             let v = v.downcast_ref::<bool>().unwrap();
//             let old = self.0.insert(k, FieldValue::t(v.clone()));
//         } else if v.is::<i8>() {
//             let v = v.downcast_ref::<i8>().unwrap();
//             let old = self.0.insert(k, FieldValue::b(v.clone()));
//         } else if v.is::<u8>() {
//             let v = v.downcast_ref::<u8>().unwrap();
//             let old = self.0.insert(k, FieldValue::B(v.clone()));
//         } else if v.is::<i16>() {
//             let v = v.downcast_ref::<bool>().unwrap();
//             let old = self.0.insert(k, FieldValue::t(v.clone()));
//         } else if v.is::<u16>() {
//         } else if v.is::<i32>() {
//         } else if v.is::<u32>() {
//         } else if v.is::<i64>() {
//         } else if v.is::<f32>() {
//         } else if v.is::<f64>() {
//         } else if v.is::<DecimalValue>() {
//         } else if v.is::<String>() {
//             // RabbitMQ does not have "short string" type in field value,
//             let v = v.downcast_ref::<String>().unwrap();
//             let old = self
//                 .0
//                 .insert(k, FieldValue::S(v.clone().try_into().unwrap()));
//         } else if v.is::<FieldArray>() {
//         } else if v.is::<u64>() { // RabbitMQ do not have "Unsigned 64-bit" field value, so `u64` can be uniquely mapped to TimeStamp
//         } else if v.is::<Self>() {
//         } else if v.is::<()>() {
//         } else if v.is::<ByteArray>() {
//         } else {
//             panic!("unsupported value type {:?} ", v);
//         }
        
//     }
// }
/////////////////////////////////////////////////////////////////////////////
/// RabbitMQ use LongUint as length value
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
pub type AmqpChannelId = ShortUint; // Define it as type used as `channel id` in AMQP frame instead of `longstr` in amqp-rabbitmq-0.9.1.json which is only used for `open-ok`
pub type AmqpClassId = ShortUint;
pub type AmqpMethodId = ShortUint;

pub type AmqpConsumerTag = ShortStr;
pub type AmqpDeliveryTag = LongLongUint;
pub type AmqpDestination = ShortStr;
pub type AmqpDuration = LongLongUint;
pub type AmqpExchangeName = ShortStr;
pub type AmqpMessageCount = LongUint;
pub type AmqpOffset = LongUint;
pub type AmqpPath = ShortStr;
pub type AmqpPeerProperties = FieldTable;
pub type AmqpQueueName = ShortStr;
pub type AmqpReference = LongStr;
pub type AmqpRejectCode = ShortUint;
pub type AmqpRejectText = ShortStr;
pub type AmqpReplyCode = ShortUint;
pub type AmqpReplyText = ShortStr;
pub type AmqpSecurityToken = LongStr;
pub type AmqpTable = FieldTable;
pub type AmqpTimeStamp = TimeStamp;

/////////////////////////////////////////////////////////////////////////////
mod tests {
    use super::FieldTable;
    #[test]
    fn test() {
        let mut table = FieldTable::new();
        // table.insert("hello".to_string(), "world".to_string());
        println!("{:?}", table);
    }
}
