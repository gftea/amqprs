//! AMQP 0-9-1 types definition and implementation.
//!
//! See [RabbitMQ's Definition](https://github.com/rabbitmq/rabbitmq-codegen/blob/main/amqp-rabbitmq-0.9.1.json).
//!
//! See [RabbitMQ errata](https://www.rabbitmq.com/amqp-0-9-1-errata.html)
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{self, Debug},
    mem,
    num::TryFromIntError,
};

/// DO NOT USE. No primitive rust type to represent single bit.
///
/// Bits are packed in octect according to AMQP 0-9-1 protocol.
pub type Bit = u8;

pub type Octect = u8;
pub type Boolean = bool; // 0 = FALSE, else TRUE. `bool` size is 1 byte in Rust
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
/// // convert ShortStr to String
/// let s: String = s.into();
/// ```
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct ShortStr(u8, String);

impl ShortStr {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn len_in_bytes(&self) -> usize {
        // size of length field + size of content
        mem::size_of_val(&self.0) + self.1.len()
    }
}
impl fmt::Display for ShortStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.1)
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
///
/// // create a LongStr from `String`
/// let s: LongStr = String::from("hello").try_into().unwrap();
///
/// // convert LongStr to String
/// let s: String = s.into();
/// ```
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct LongStr(u32, String);

impl LongStr {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn len_in_bytes(&self) -> usize {
        // size of length field + size of content
        mem::size_of_val(&self.0) + self.1.len()
    }
}
impl fmt::Display for LongStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.1)
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
///
/// # Usage
///
/// ```
/// # use amqp_serde::types::{DecimalValue, Octect, LongInt};
///
/// let decimal = DecimalValue::new(0, 12345);
/// assert_eq!("12345", decimal.to_string());
///
/// let decimal = DecimalValue::new(2, 12345);
/// assert_eq!("123.45", decimal.to_string());
///
/// let decimal = DecimalValue::new(5, 12345);
/// assert_eq!("0.12345", decimal.to_string());
///
/// let decimal = DecimalValue::new(8, 12345);
/// assert_eq!("0.00012345", decimal.to_string());
/// ```
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct DecimalValue(Octect, LongInt);

impl DecimalValue {
    pub fn new(scale: Octect, value: LongInt) -> Self {
        Self(scale, value)
    }

    #[inline]
    fn len_in_bytes(&self) -> usize {
        mem::size_of_val(&self.0) + mem::size_of_val(&self.1)
    }
}

impl fmt::Display for DecimalValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scale = self.0 as usize;
        if scale == 0 {
            return write!(f, "{}", self.1);
        }
        let sign = if self.1 < 0 { "-" } else { "" };
        let value = self.1.abs().to_string();

        if scale < value.len() {
            let (int, frac) = value.split_at(value.len() - scale);
            write!(f, "{}{}.{}", sign, int, frac)
        } else {
            write!(f, "{}0.{:0>width$}", sign, value, width = scale)
        }
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
    // Every field-value serialized with a type tag of one byte size
    const TYPE_TAG_SIZE: usize = 1;

    fn len_in_bytes(&self) -> usize {
        match self {
            Self::V => 0,                                                         // fixed size
            Self::t(_) => Self::TYPE_TAG_SIZE + mem::size_of::<Boolean>(),        // fixed size
            Self::b(_) => Self::TYPE_TAG_SIZE + mem::size_of::<ShortShortInt>(),  // fixed size
            Self::B(_) => Self::TYPE_TAG_SIZE + mem::size_of::<ShortShortUint>(), // fixed size
            Self::s(_) => Self::TYPE_TAG_SIZE + mem::size_of::<ShortInt>(),       // fixed size
            Self::u(_) => Self::TYPE_TAG_SIZE + mem::size_of::<ShortUint>(),      // fixed size
            Self::I(_) => Self::TYPE_TAG_SIZE + mem::size_of::<LongInt>(),        // fixed size
            Self::i(_) => Self::TYPE_TAG_SIZE + mem::size_of::<LongUint>(),       // fixed size
            Self::l(_) => Self::TYPE_TAG_SIZE + mem::size_of::<LongLongInt>(),    // fixed size
            Self::f(_) => Self::TYPE_TAG_SIZE + mem::size_of::<Float>(),          // fixed size
            Self::d(_) => Self::TYPE_TAG_SIZE + mem::size_of::<Double>(),         // fixed size
            Self::T(_) => Self::TYPE_TAG_SIZE + mem::size_of::<TimeStamp>(),      // fixed size
            Self::D(v) => Self::TYPE_TAG_SIZE + v.len_in_bytes(),                 // fixed size
            Self::S(v) => Self::TYPE_TAG_SIZE + v.len_in_bytes(),                 // variable size
            Self::A(v) => Self::TYPE_TAG_SIZE + v.len_in_bytes(),                 // variable size
            Self::F(v) => Self::TYPE_TAG_SIZE + v.len_in_bytes(),                 // variable size
            Self::x(v) => Self::TYPE_TAG_SIZE + v.len_in_bytes(),                 // variable size
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
            _ => Err(crate::Error::Message("not a boolean".to_string())),
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
            FieldValue::V => write!(f, ""),
            FieldValue::x(v) => write!(f, "{}", v),
        }
    }
}
//////////////////////////////////////////////////////////////////////////////

/// AMQP field-table type that is serializable and deserializable.
/// `FieldTable` is immutable after creation.
///
/// Use `HashMap<FieldName, FieldValue>` to construct a table, then convert it to `FieldTable` before serialization.
/// After deserializing a `FieldTable` from bytes, convert it to `HashMap<FieldName, FieldValue>` if you need to modify it.
///
/// Every time converting a `HashMap<FieldName, FieldValue>` to `FieldTable`, it will calculate the size of the table, and
/// return an error if the size is over AMQP's limit. So, the `FiledTable` is guaranteed to be serialized without overflow.
/// This simplifies the serialization process.
///
/// Calculating the size of `FieldTable` is O(n) where n is the number of fields, so it is recommended to finish all
/// modifications to the `HashMap` before converting it to `FieldTable`, avoid unnecessary conversion back and forth.
///
/// # Usage
///
/// ```
/// # use amqp_serde::types::{FieldTable, FieldName, FieldValue};
/// # use std::collections::HashMap;
///
/// let mut map = HashMap::new();
/// map.insert("foo".try_into().unwrap(), FieldValue::t(true));
/// map.insert("bar".try_into().unwrap(), FieldValue::i(42));
/// let table = FieldTable::try_from(map).unwrap();
///
/// let map: HashMap<FieldName, FieldValue> = table.into();
/// ```
///
///
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct FieldTable(HashMap<FieldName, FieldValue>);
pub type FieldName = ShortStr;

impl FieldTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// calculate number of bytes required to serialize field table.
    #[inline]
    fn calculate_bytes(v: &HashMap<FieldName, FieldValue>) -> usize {
        v.iter()
            .fold(0, |acc, (k, v)| acc + k.len_in_bytes() + v.len_in_bytes())
    }

    #[inline]
    fn len_in_bytes(&self) -> usize {
        Self::calculate_bytes(&self.0)
    }
}
impl fmt::Display for FieldTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| (k.to_string(), v.to_string())))
            .finish()
    }
}

impl TryFrom<HashMap<FieldName, FieldValue>> for FieldTable {
    type Error = TryFromIntError;

    fn try_from(v: HashMap<FieldName, FieldValue>) -> Result<Self, Self::Error> {
        LongUint::try_from(Self::calculate_bytes(&v)).map(|_| Self(v))
    }
}

/// Convert to `HashMap` from `FieldTable`.
impl From<FieldTable> for HashMap<FieldName, FieldValue> {
    fn from(v: FieldTable) -> Self {
        v.0
    }
}

impl AsRef<HashMap<FieldName, FieldValue>> for FieldTable {
    fn as_ref(&self) -> &HashMap<FieldName, FieldValue> {
        &self.0
    }
}

/////////////////////////////////////////////////////////////////////////////
/// AMQP byte array type.

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct ByteArray(Vec<u8>);

impl ByteArray {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn len_in_bytes(&self) -> usize {
        self.0.len()
    }
}

impl TryFrom<Vec<u8>> for ByteArray {
    type Error = TryFromIntError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        LongUint::try_from(bytes.len()).map(|_| Self(bytes))
    }
}
impl From<ByteArray> for Vec<u8> {
    fn from(arr: ByteArray) -> Self {
        arr.0
    }
}
impl fmt::Display for ByteArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}
/////////////////////////////////////////////////////////////////////////////
/// AMQP field array type.
///
/// Same design as `FieldTable`.

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct FieldArray(Vec<FieldValue>); // RabbitMQ use LongUint as length value

impl FieldArray {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline]
    fn calculate_bytes(arr: &[FieldValue]) -> usize {
        arr.iter().fold(0, |acc, v| acc + v.len_in_bytes())
    }

    #[inline]
    fn len_in_bytes(&self) -> usize {
        Self::calculate_bytes(&self.0)
    }
}

impl fmt::Display for FieldArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|v| v.to_string()))
            .finish()
    }
}

impl TryFrom<Vec<FieldValue>> for FieldArray {
    type Error = TryFromIntError;

    fn try_from(v: Vec<FieldValue>) -> Result<Self, Self::Error> {
        LongUint::try_from(Self::calculate_bytes(&v)).map(|_| Self(v))
    }
}

impl From<FieldArray> for Vec<FieldValue> {
    fn from(arr: FieldArray) -> Self {
        arr.0
    }
}

impl AsRef<Vec<FieldValue>> for FieldArray {
    fn as_ref(&self) -> &Vec<FieldValue> {
        &self.0
    }
}

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
    use std::collections::HashMap;

    use crate::types::{ByteArray, DecimalValue, FieldArray, FieldValue, LongStr};

    use super::{FieldTable, ShortStr};
    #[test]
    fn test_field_table() {
        let mut table = HashMap::new();
        table.insert(
            "Cash".try_into().unwrap(), // Size (1 byte) + "Cash" (4 bytes) = 5 bytes
            FieldValue::D(DecimalValue(3, -123456)), // Type (1 byte) + 1 Octect + LongUint (4 bytes) = 6 bytes
        );
        let table: FieldTable = table.try_into().unwrap();
        assert_eq!(11, table.len_in_bytes());
        assert_eq!("{\"Cash\": \"-123.456\"}", format!("{}", table));
    }

    #[test]
    fn test_field_array() {
        let exp = vec![FieldValue::t(true), FieldValue::D(DecimalValue(3, 123456))];
        let field_arr: FieldArray = exp.clone().try_into().unwrap();
        assert_eq!("[\"true\", \"123.456\"]", format!("{}", field_arr));
        let arr: Vec<FieldValue> = field_arr.into();
        assert_eq!(exp, arr);
    }

    #[test]
    fn test_bytes_array() {
        let exp: Vec<u8> = vec![1, 2, 3];
        let bytes_arr: ByteArray = exp.clone().try_into().unwrap();
        assert_eq!(3, bytes_arr.len_in_bytes());

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
}
