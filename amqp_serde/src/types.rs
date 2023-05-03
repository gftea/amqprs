//! AMQP 0-9-1 types definition and implementation.
//!
//! See [RabbitMQ's Definition](https://github.com/rabbitmq/rabbitmq-codegen/blob/main/amqp-rabbitmq-0.9.1.json).
//!
//! See [RabbitMQ errata](https://www.rabbitmq.com/amqp-0-9-1-errata.html)
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{self, Debug},
    mem::{size_of, size_of_val},
    num::TryFromIntError,
};
use serde::{Deserialize, Serialize};

/// DO NOT USE. No primitive rust type to represent single bit.
///
/// Bits are packed in octect according to AMQP 0-9-1 protocol.
pub type Bit = u8;

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
/// AMQP short string type.
///
/// User should not directly create it, but use conversion method to create
/// from `String` or `&str`.
///
/// # Usage
///
/// ```
/// # use amqp_serde::types::ShortStr;
/// // create a ShortStr from &str
/// let s: ShortStr = "hello".try_into().unwrap();
///
/// // create a ShortStr from String
/// let s: ShortStr = String::from("hello").try_into().unwrap();
///
/// ```
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ShortStr(u8, String);

impl fmt::Display for ShortStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
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
impl AsRef<String> for ShortStr {
    fn as_ref(&self) -> &String {
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
/// AMQP long string type.
///
/// User should not directly create it, but use conversion method to create
/// from `String` or `&str`.
///
/// # Usage
///
/// ```
/// # use amqp_serde::types::LongStr;
/// // create a LongStr from `&str`
/// let s: LongStr = "hello".try_into().unwrap();
/// // create a LongStr from `String`
/// let s: LongStr = String::from("hello").try_into().unwrap();
/// ```
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct LongStr(u32, String);

impl fmt::Display for LongStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
impl Default for LongStr {
    fn default() -> Self {
        Self(0, "".to_string())
    }
}

impl AsRef<String> for LongStr {
    fn as_ref(&self) -> &String {
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
/// AMQP decimal type.
///
/// decimal-value = "scale long-int".
///
/// RabbitMQ treat the decimal value as signed integer.
/// See notes "Decimals encoding" in [amqp-0-9-1-errata](https://www.rabbitmq.com/amqp-0-9-1-errata.html).
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct DecimalValue(Octect, LongInt);

impl DecimalValue {
    pub fn new(scale: Octect, value: LongInt) -> Self {
        Self(scale, value)
    }
}

impl fmt::Display for DecimalValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decimal({}, {})", self.0, self.1)
    }
}

/////////////////////////////////////////////////////////////////////////////
/// AMQP byte array type.

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
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
impl fmt::Display for ByteArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.1)
    }
}
/////////////////////////////////////////////////////////////////////////////

/// AMQP field array type.

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct FieldArray(LongUint, Vec<FieldValue>); // RabbitMQ use LongUint as length value

impl FieldArray {
    pub fn new() -> Self {
        Self(0, Vec::with_capacity(0))
    }
}
impl Default for FieldArray {
    fn default() -> Self {
        Self::new()
    }
}
impl TryFrom<Vec<FieldValue>> for FieldArray {
    type Error = TryFromIntError;

    fn try_from(values: Vec<FieldValue>) -> Result<Self, Self::Error> {
        let total_bytes = values.iter().fold(0, |acc, v| acc + FieldValue::TAG_SIZE + v.len());
        let len = LongUint::try_from(total_bytes)?;
        Ok(Self(len, values))
    }
}
impl From<FieldArray> for Vec<FieldValue> {
    fn from(arr: FieldArray) -> Self {
        arr.1
    }
}

impl fmt::Display for FieldArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ ")?;
        let len = self.1.len();
        for v in self.1.iter().take(len - 1) {
            write!(f, "{}, ", v)?;
        }
        if let Some(v) = self.1.last() {
            write!(f, "{} ", v)?;
        }

        write!(f, "]")?;
        Ok(())
    }
}
//////////////////////////////////////////////////////////////////////////////
/// AMQP field value type.
///
/// User is recommended to use conversion method to create FieldValue from rust's type.
///
/// See [RabbitMQ errata](https://www.rabbitmq.com/amqp-0-9-1-errata.html#section_3).
///
/// # Usage
///
/// ```
/// # use amqp_serde::types::FieldValue;
/// // convert from `bool`
/// let x: FieldValue = true.into();
///
/// // convert into `bool`
/// let y: bool = x.try_into().unwrap();
/// ```
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

impl FieldValue {
    const TAG_SIZE: usize = 1;

    fn len(&self) -> usize {
        match self {
            Self::V => 0, // fixed size
            Self::t(_) => size_of::<Boolean>(), // fixed size
            Self::b(_) => size_of::<ShortShortInt>(), // fixed size
            Self::B(_) => size_of::<ShortShortUint>(), // fixed size
            Self::s(_) => size_of::<ShortInt>(), // fixed size
            Self::u(_) => size_of::<ShortUint>(), // fixed size
            Self::I(_) => size_of::<LongInt>(), // fixed size
            Self::i(_) => size_of::<LongUint>(), // fixed size
            Self::l(_) => size_of::<LongLongInt>(), // fixed size
            Self::f(_) => size_of::<Float>(), // fixed size
            Self::d(_) => size_of::<Double>(), // fixed size
            Self::T(_) => size_of::<TimeStamp>(), // fixed size
            Self::D(v) => size_of_val(&v.0) + size_of_val(&v.1), // fixed size
            Self::S(v) => size_of_val(&v.0) + v.0 as usize, // variable size
            Self::A(v) => size_of_val(&v.0) + v.0 as usize, // variable size
            Self::F(v) => size_of_val(&v.0) + v.0 as usize, // variable size
            Self::x(v) => size_of_val(&v.0) + v.0 as usize, // variable size
        }
    }
}

impl From<bool> for FieldValue {
    fn from(v: bool) -> Self {
        FieldValue::t(v)
    }
}
impl TryInto<bool> for FieldValue {
    type Error = crate::Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self {
            FieldValue::t(v) => Ok(v),
            _ => Err(crate::Error::Message("not a bool".to_string())),
        }
    }
}
impl From<FieldTable> for FieldValue {
    fn from(v: FieldTable) -> Self {
        FieldValue::F(v)
    }
}
impl TryInto<FieldTable> for FieldValue {
    type Error = crate::Error;

    fn try_into(self) -> Result<FieldTable, Self::Error> {
        match self {
            FieldValue::F(v) => Ok(v),
            _ => Err(crate::Error::Message("not a FieldTable".to_string())),
        }
    }
}

impl From<LongStr> for FieldValue {
    fn from(v: LongStr) -> Self {
        FieldValue::S(v)
    }
}

impl TryInto<LongStr> for FieldValue {
    type Error = crate::Error;

    fn try_into(self) -> Result<LongStr, Self::Error> {
        match self {
            FieldValue::S(v) => Ok(v),
            _ => Err(crate::Error::Message("not a LongStr".to_string())),
        }
    }
}

/// RabbitMQ's field value support only long string variant, so rust string type
/// always converted to long string variant.
impl From<String> for FieldValue {
    fn from(v: String) -> Self {
        FieldValue::S(v.try_into().unwrap())
    }
}

impl TryInto<String> for FieldValue {
    type Error = crate::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            FieldValue::S(v) => Ok(v.into()),
            _ => Err(crate::Error::Message("not a LongStr".to_string())),
        }
    }
}

impl From<&str> for FieldValue {
    fn from(v: &str) -> Self {
        FieldValue::S(v.try_into().unwrap())
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::t(v) => write!(f, "{}", v),
            FieldValue::b(v) => write!(f, "{}", v),
            FieldValue::B(v) => write!(f, "{}", v),
            FieldValue::s(v) => write!(f, "{}", v),
            FieldValue::u(v) => write!(f, "{}", v),
            FieldValue::I(v) => write!(f, "{}", v),
            FieldValue::i(v) => write!(f, "{}", v),
            FieldValue::l(v) => write!(f, "{}", v),
            FieldValue::f(v) => write!(f, "{}", v),
            FieldValue::d(v) => write!(f, "{}", v),
            FieldValue::D(v) => write!(f, "{}", v),
            FieldValue::S(v) => write!(f, "{}", v),
            FieldValue::A(v) => write!(f, "{}", v),
            FieldValue::T(v) => write!(f, "{}", v),
            FieldValue::F(v) => write!(f, "{}", v),
            FieldValue::V => write!(f, "()"),
            FieldValue::x(v) => write!(f, "{}", v),
        }
    }
}
//////////////////////////////////////////////////////////////////////////////

pub type FieldName = ShortStr;
/// AMQP field table type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct FieldTable(LongUint, HashMap<FieldName, FieldValue>);

impl FieldTable {
    pub fn new() -> Self {
        Self(0, HashMap::new())
    }

    pub fn insert(&mut self, k: FieldName, v: FieldValue) -> Option<FieldValue> {
        self.0 += LongUint::try_from(size_of_val(&k.0) + k.0 as usize + FieldValue::TAG_SIZE + v.len()).unwrap();
        self.1.insert(k, v)
    }

    pub fn remove(&mut self, k: &FieldName) -> Option<FieldValue> {
        if let Some(v) = self.1.remove(k) {
            self.0 -= LongUint::try_from(size_of_val(&k.0) + k.0 as usize + FieldValue::TAG_SIZE + v.len()).unwrap();
            Some(v)
        } else {
            None
        }
    }

    pub fn get(&self, k: &FieldName) -> Option<&FieldValue> {
        self.1.get(k)
    }
}
impl fmt::Display for FieldTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ ")?;
        if self.1.len() > 1 {
            for (k, v) in self.1.iter().take(self.1.len() - 1) {
                write!(f, "{}: {}, ", k, v)?;
            }
        }
        if let Some((k, v)) = self.1.iter().last() {
            write!(f, "{}: {} ", k, v)?;
        }

        write!(f, "}}")?;
        Ok(())
    }
}

impl AsRef<HashMap<FieldName, FieldValue>> for FieldTable {
    fn as_ref(&self) -> &HashMap<FieldName, FieldValue> {
        &self.1
    }
}

/////////////////////////////////////////////////////////////////////////////
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
// AMQP domains
/// Note: it is different from definition in [`RabbitMQ Definition`].
///
/// In [`RabbitMQ Definition`], it is defined as `longstr`, and only used in `open-ok` frame.
///
/// Here, it is defined as [`ShortUint`], which is the type of channel id field in AMQP frame.
/// It is intended to be used in place where readability can be improved.
///
/// [`RabbitMQ Definition`]: https://github.com/rabbitmq/rabbitmq-codegen/blob/main/amqp-rabbitmq-0.9.1.json
pub type AmqpChannelId = ShortUint;

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
#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use crate::types::{ByteArray, DecimalValue, FieldArray, FieldValue, LongStr, LongUint};

    use super::{FieldTable, ShortStr};
    #[test]
    fn test_field_table() {
        let mut table = FieldTable::new();
        table.insert(
            "Cash".try_into().unwrap(), // Size (1 byte) + "Cash" (4 bytes) = 5 bytes
            FieldValue::D(DecimalValue(3, 123456)), // Type (1 byte) + 1 Octect + LongUint (4 bytes) = 6 bytes
        );

        assert_eq!(11, table.0);
        assert_eq!("{ Cash: Decimal(3, 123456) }", format!("{}", table));
    }

    #[test]
    fn test_field_array() {
        let exp = vec![FieldValue::t(true), FieldValue::D(DecimalValue(3, 123456))];
        let field_arr: FieldArray = exp.clone().try_into().unwrap();
        assert_eq!("[ true, Decimal(3, 123456) ]", format!("{}", field_arr));

        let arr: Vec<FieldValue> = field_arr.into();
        assert_eq!(exp, arr);
    }

    #[test]
    fn test_bytes_array() {
        let exp: Vec<u8> = vec![1, 2, 3];
        let bytes_arr: ByteArray = exp.clone().try_into().unwrap();
        assert_eq!(3, bytes_arr.0);

        let arr: Vec<u8> = bytes_arr.into();
        assert_eq!(exp, arr);
    }

    #[test]
    fn test_shortstr() {
        let s = ShortStr::default();
        assert_eq!(ShortStr(0, "".to_owned()), s);

        let exp = "x".repeat(255);
        // from str to shortstr
        let s: ShortStr = exp.clone().try_into().unwrap();
        assert_eq!(255, s.0);
        // from shortstr to str
        let s: String = s.into();
        assert_eq!(exp, s);
    }

    #[test]
    fn test_longstr() {
        let s = LongStr::default();
        assert_eq!(LongStr(0, "".to_owned()), s);

        let exp = "x".repeat(256);
        // from str to shortstr
        let s: LongStr = exp.clone().try_into().unwrap();
        assert_eq!(256, s.0);
        // from shortstr to str
        let s: String = s.into();
        assert_eq!(exp, s);
    }

    #[test]
    fn test_field_value() {
        let exp = FieldValue::t(true);
        assert_eq!(exp, true.into());
        let t: bool = exp.try_into().unwrap();
        assert_eq!(true, t);

        let exp = FieldValue::F(FieldTable::default());
        assert_eq!(exp, FieldTable::default().into());
        let t: FieldTable = exp.try_into().unwrap();
        assert_eq!(FieldTable::default(), t);

        let exp = FieldValue::S("X".to_owned().try_into().unwrap());
        assert_eq!(exp, "X".to_owned().into());
        let t: String = exp.try_into().unwrap();
        assert_eq!("X".to_owned(), t);
    }

    #[test]
    fn test_len_for_field_value_of_type_field_array() {
        let s1: LongStr = "1".try_into().unwrap();
        let s2: LongStr = "2".try_into().unwrap();
        let s3: LongStr = "3".try_into().unwrap();
        let s4: LongStr = "finally".try_into().unwrap();
        let mut total_length = s1.1.len() + s2.1.len() + s3.1.len() + s4.1.len();
        let v = vec![
            FieldValue::S(s1),
            FieldValue::S(s2),
            FieldValue::S(s3),
            FieldValue::S(s4),
        ];

        v.iter().for_each(|_fv| total_length += FieldValue::TAG_SIZE + size_of::<LongUint>());

        let a = FieldArray::try_from(v).unwrap();

        total_length += size_of::<LongUint>();

        let exp = FieldValue::A(a);
        assert_eq!(exp.len(), total_length);
    }

    #[test]
    fn test_len_for_field_value_of_type_field_table() {
        let mut ft = FieldTable::new();
        let v_kv: Vec<(ShortStr, LongStr)> = vec![
            ("key1".try_into().unwrap(), "value1".try_into().unwrap()),
            ("key2".try_into().unwrap(), "value2".try_into().unwrap()),
            ("key3".try_into().unwrap(), "value3".try_into().unwrap()),
            ("key4".try_into().unwrap(), "value4".try_into().unwrap()),
            ("key5".try_into().unwrap(), "value5".try_into().unwrap()),
            ("key6".try_into().unwrap(), "value6".try_into().unwrap()),
            ("key7".try_into().unwrap(), "value7".try_into().unwrap()),
        ];
        let mut total_length = 0;
        v_kv.into_iter().for_each(|(k, v)| {
            total_length += k.1.len() + v.1.len() + 2 * FieldValue::TAG_SIZE + size_of::<LongUint>();
            ft.insert(k, FieldValue::S(v));
        });

        total_length += size_of::<LongUint>();

        let exp = FieldValue::F(ft);
        assert_eq!(exp.len(), total_length);
    }

    #[test]
    fn test_len_for_field_value_of_type_bytearray() {
        let v = b"This is un example".to_vec();
        let length = v.len() + size_of::<LongUint>();
        let ba: ByteArray = v.try_into().unwrap();
        let exp = FieldValue::x(ba);

        assert_eq!(exp.len(), length);
    }
}
