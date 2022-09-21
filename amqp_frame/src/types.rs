use std::collections::HashMap;

use serde::Serialize;
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

#[derive(Serialize, PartialEq, Eq, Hash)]
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

#[derive(Serialize)]
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

// field-value = 't' boolean
//               'b' short-short-int
//               'B' short-short-uint
//               'U' short-int
//               'u' short-uint
//               'I' long-int
//               'i' long-uint
//               'L' long-long-int
//               'l' long-long-uint
//               'f' float
//               'd' double
//               'D' decimal-value
//               's' short-string
//               'S' long-string
//               'A' field-array
//               'T' timestamp
//               'F' field-table
//               'V' ; no field
// #[derive(Serialize)]

#[derive(Serialize)]
pub struct DecimalValue(Octect, LongUint);

#[derive(Serialize)]
pub enum FieldValue {
    t(Boolean),
    b(ShortShortInt),
    B(ShortShortUint),
    U(ShortInt),
    u(ShortUint),
    I(LongInt),
    i(LongUint),
    L(LongLongInt),
    l(LongLongUint),
    f(Float),
    d(Double),
    D(DecimalValue),
    s(ShortStr),
    S(LongStr),
    A(FieldArray),
    T(TimeStamp),
    F(FieldTable),
    V,
}
pub type FieldName = ShortStr;
pub type FieldTable = HashMap<FieldName, FieldValue>;

pub type FieldArray = (LongInt, Vec<FieldValue>);
