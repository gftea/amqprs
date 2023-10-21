// Copyright 2018 Serde Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::error::{Error, Result};

use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};

pub struct Deserializer<'de> {
    input: &'de [u8],
    last_parsed_len: Option<usize>,
    cursor: usize,
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.

    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer {
            input,
            last_parsed_len: None,
            cursor: 0,
        }
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::Incomplete)
    }
}

/////////////////////////////////////////////////////////////////////////////
macro_rules! impl_inner {
    ($self:ident, $typ:tt, $($index:literal),+) => {{
        let size = std::mem::size_of::<$typ>();
        $self.last_parsed_len = None;
        $self.cursor += size;
        if $self.input.len() < size {
            Err(Error::Eof)
        } else {
            let bytes = [$($self.input[$index],)+];
            $self.input = &$self.input[size..];
            Ok(<$typ>::from_be_bytes(bytes))
        }}
    };
}
macro_rules! impl_parse_num {
    ($func_name:ident, $typ:tt, $($index:literal),+) => {
        fn $func_name(&mut self) -> Result<$typ> {
            let res = impl_inner!(self, $typ, $($index),+);
            res
        }
    };
}

/////////////////////////////////////////////////////////////////////////////

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    // Look at the first byte in the input without consuming it.
    fn peek_byte(&mut self) -> Result<u8> {
        self.input.iter().next().copied().ok_or(Error::Eof)
    }

    // Consume the first byte in the input.
    fn next_byte(&mut self) -> Result<u8> {
        let v = self.peek_byte()?;
        self.input = &self.input[1..];
        self.cursor += 1;
        Ok(v)
    }

    fn parse_bool(&mut self) -> Result<bool> {
        let v = self.next_byte()?;
        if v > 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    impl_parse_num!(parse_i8, i8, 0);
    impl_parse_num!(parse_i16, i16, 0, 1);
    impl_parse_num!(parse_i32, i32, 0, 1, 2, 3);
    impl_parse_num!(parse_i64, i64, 0, 1, 2, 3, 4, 5, 6, 7);
    impl_parse_num!(parse_u16, u16, 0, 1);
    impl_parse_num!(parse_u64, u64, 0, 1, 2, 3, 4, 5, 6, 7);
    impl_parse_num!(parse_f32, f32, 0, 1, 2, 3);
    impl_parse_num!(parse_f64, f64, 0, 1, 2, 3, 4, 5, 6, 7);

    // valid integer type as length for ShortStr
    fn parse_u8(&mut self) -> Result<u8> {
        let res = impl_inner!(self, u8, 0);
        if let Ok(v) = res {
            self.last_parsed_len = Some(v as usize);
        }
        res
    }

    // valid interger type as length for LongStr
    fn parse_u32(&mut self) -> Result<u32> {
        let res = impl_inner!(self, u32, 0, 1, 2, 3);
        if let Ok(v) = res {
            self.last_parsed_len = Some(v as usize);
        }
        res
    }

    fn get_parsed_length(&self) -> Result<usize> {
        match self.last_parsed_len {
            Some(len) if len <= u32::MAX as usize => {
                let len = len;
                if self.input.len() < len {
                    Err(Error::Syntax)
                } else {
                    Ok(len)
                }
            }
            _ => Err(Error::ExpectedLength),
        }
    }

    fn parse_string(&mut self) -> Result<&'de str> {
        let len = self.get_parsed_length()?;
        self.cursor += len;

        let s = &self.input[..len];
        self.input = &self.input[len..];
        std::str::from_utf8(s)
            .map_err(|_| Error::Message(format!("len = {}, content = {:02X?}", len, s)))
    }

    fn next_bytes(&mut self) -> Result<&'de [u8]> {
        let len = self.get_parsed_length()?;
        self.cursor += len;

        let s = &self.input[..len];
        self.input = &self.input[len..];
        Ok(s)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // Uses the `parse_bool` parsing function defined above to read the JSON
    // identifier `true` or `false` from the input.
    //
    // Parsing refers to looking at the input and deciding that it contains the
    // JSON value `true` or `false`.
    //
    // Deserialization refers to mapping that JSON value into Serde's data
    // model by invoking one of the `Visitor` methods. In the case of JSON and
    // bool that mapping is straightforward so the distinction may seem silly,
    // but in other cases Deserializers sometimes perform non-obvious mappings.
    // For example the TOML format has a Datetime type and Serde's data model
    // does not. In the `toml` crate, a Datetime in the input is deserialized by
    // mapping it to a Serde data model "struct" type with a special name and a
    // single field containing the Datetime represented as a string.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    // The `parse_signed` function is generic over the integer type `T` so here
    // it is invoked with `T=i8`. The next 8 methods are similar.
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_i8()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_i16()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_i32()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_i64()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_u8()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_u16()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_u32()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_u64()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse_f32()?)
    }

    // Float parsing is stupidly hard.
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_f64()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // char is 4 bytes, either it is a ShortStr or LongStr
        // length should already parsed just before
        self.deserialize_str(visitor)
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // either a ShortStr or LongStr
        // length should already parsed just before
        visitor.visit_borrowed_str(self.parse_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // either a ShortStr or LongStr
        // length should already parsed just before
        self.deserialize_str(visitor)
    }

    // no length input, so bytes expect length prefix
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.next_bytes()?)
    }

    // no length input, so expect length prefix
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.next_bytes()?.to_owned())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // no length input, so expect length prefix
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // should have a length parsed right before
        let len = self.get_parsed_length()?;
        visitor.visit_seq(DataSequence::new(self, len))
    }

    // given length input, do not need length prefix in input data
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(DataSequence::new_struct(self, len))
    }

    // given length input, do not need length prefix in input data
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(DataSequence::new_struct(self, len))
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // should have a length parsed right before
        // the length is number of bytes of the table, not the field-value pair
        let len = self.get_parsed_length()?;
        visitor.visit_map(DataSequence::new(self, len))
    }

    // length can be derived by fields
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(DataSequence::new_struct(self, fields.len()))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(Enum::new(self))
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

// In order to handle commas correctly when deserializing a JSON array or map,
// we need to track whether we are on the first element or past the first
// element.
struct DataSequence<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: usize,
    is_struct: bool,
}

impl<'a, 'de> DataSequence<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, len: usize) -> Self {
        DataSequence {
            de,
            len,
            is_struct: false,
        }
    }

    fn new_struct(de: &'a mut Deserializer<'de>, len: usize) -> Self {
        DataSequence {
            de,
            len,
            is_struct: true,
        }
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for DataSequence<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            if self.is_struct {
                self.len -= 1;
                seed.deserialize(&mut *self.de).map(Some)
            } else {
                let start = self.de.cursor;
                let res = seed.deserialize(&mut *self.de).map(Some);
                let end = self.de.cursor;
                self.len -= end - start;
                res
            }
        } else {
            Ok(None)
        }
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for DataSequence<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            let start = self.de.cursor;
            let res = seed.deserialize(&mut *self.de).map(Some);
            let end = self.de.cursor;
            self.len -= end - start;
            res
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let start = self.de.cursor;
        let res = seed.deserialize(&mut *self.de);
        let end = self.de.cursor;
        self.len -= end - start;
        res
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        match self.de.next_byte()? {
            v if v.is_ascii_alphabetic() => {
                let val = [v];

                let variant = unsafe { std::str::from_utf8_unchecked(&val[..]) };
                let val = seed.deserialize(variant.into_deserializer())?;
                Ok((val, self))
            }
            v => panic!(
                "unsupported enum variant for AMQP field value: {}, cursor: {}",
                v, self.de.cursor
            ),
        }
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.de, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.de, fields.len(), visitor)
    }
}

/////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::de::from_bytes;
    use crate::types::*;
    use serde::Deserialize;

    #[test]
    fn test_array() {
        let input = vec![
            0x41, 0x00, 0x00, 0x01, 0x7c, 0x53, 0x00, 0x00, 0x00, 0x32, 0x7b, 0x3c, 0x3c, 0x22,
            0x63, 0x6f, 0x6e, 0x6e, 0x65, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x5f, 0x6e, 0x61, 0x6d,
            0x65, 0x22, 0x3e, 0x3e, 0x2c, 0x6c, 0x6f, 0x6e, 0x67, 0x73, 0x74, 0x72, 0x2c, 0x3c,
            0x3c, 0x22, 0x30, 0x45, 0x30, 0x45, 0x30, 0x45, 0x42, 0x30, 0x42, 0x30, 0x32, 0x30,
            0x22, 0x3e, 0x3e, 0x7d, 0x53, 0x00, 0x00, 0x00, 0x26, 0x7b, 0x3c, 0x3c, 0x22, 0x63,
            0x6f, 0x70, 0x79, 0x72, 0x69, 0x67, 0x68, 0x74, 0x22, 0x3e, 0x3e, 0x2c, 0x6c, 0x6f,
            0x6e, 0x67, 0x73, 0x74, 0x72, 0x2c, 0x3c, 0x3c, 0x22, 0x47, 0x61, 0x6c, 0x67, 0x75,
            0x73, 0x22, 0x3e, 0x3e, 0x7d, 0x53, 0x00, 0x00, 0x00, 0x25, 0x7b, 0x3c, 0x3c, 0x22,
            0x68, 0x6f, 0x73, 0x74, 0x6e, 0x61, 0x6d, 0x65, 0x22, 0x3e, 0x3e, 0x2c, 0x6c, 0x6f,
            0x6e, 0x67, 0x73, 0x74, 0x72, 0x2c, 0x3c, 0x3c, 0x22, 0x47, 0x61, 0x6c, 0x67, 0x75,
            0x73, 0x22, 0x3e, 0x3e, 0x7d, 0x53, 0x00, 0x00, 0x00, 0x21, 0x7b, 0x3c, 0x3c, 0x22,
            0x70, 0x72, 0x6f, 0x64, 0x75, 0x63, 0x74, 0x22, 0x3e, 0x3e, 0x2c, 0x6c, 0x6f, 0x6e,
            0x67, 0x73, 0x74, 0x72, 0x2c, 0x3c, 0x3c, 0x22, 0x43, 0x48, 0x54, 0x22, 0x3e, 0x3e,
            0x7d, 0x53, 0x00, 0x00, 0x00, 0x38, 0x7b, 0x3c, 0x3c, 0x22, 0x69, 0x6e, 0x66, 0x6f,
            0x72, 0x6d, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x22, 0x3e, 0x3e, 0x2c, 0x6c, 0x6f, 0x6e,
            0x67, 0x73, 0x74, 0x72, 0x2c, 0x3c, 0x3c, 0x22, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a,
            0x2f, 0x2f, 0x77, 0x77, 0x77, 0x2e, 0x67, 0x61, 0x6c, 0x67, 0x75, 0x73, 0x2e, 0x6e,
            0x65, 0x74, 0x22, 0x3e, 0x3e, 0x7d, 0x53, 0x00, 0x00, 0x00, 0x29, 0x7b, 0x3c, 0x3c,
            0x22, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x22, 0x3e, 0x3e, 0x2c, 0x6c, 0x6f,
            0x6e, 0x67, 0x73, 0x74, 0x72, 0x2c, 0x3c, 0x3c, 0x22, 0x4d, 0x6f, 0x63, 0x6b, 0x53,
            0x65, 0x72, 0x76, 0x69, 0x63, 0x65, 0x22, 0x3e, 0x3e, 0x7d, 0x53, 0x00, 0x00, 0x00,
            0x29, 0x7b, 0x3c, 0x3c, 0x22, 0x70, 0x6c, 0x61, 0x74, 0x66, 0x6f, 0x72, 0x6d, 0x22,
            0x3e, 0x3e, 0x2c, 0x6c, 0x6f, 0x6e, 0x67, 0x73, 0x74, 0x72, 0x2c, 0x3c, 0x3c, 0x22,
            0x41, 0x58, 0x31, 0x38, 0x30, 0x30, 0x59, 0x43, 0x58, 0x54, 0x22, 0x3e, 0x3e, 0x7d,
            0x53, 0x00, 0x00, 0x00, 0x2c, 0x7b, 0x3c, 0x3c, 0x22, 0x65, 0x63, 0x6f, 0x76, 0x65,
            0x72, 0x73, 0x69, 0x6f, 0x6e, 0x22, 0x3e, 0x3e, 0x2c, 0x6c, 0x6f, 0x6e, 0x67, 0x73,
            0x74, 0x72, 0x2c, 0x3c, 0x3c, 0x22, 0x4d, 0x6f, 0x63, 0x6b, 0x53, 0x65, 0x72, 0x76,
            0x69, 0x63, 0x65, 0x22, 0x3e, 0x3e, 0x7d,
        ];
        let f_array = FieldArray::try_from(vec![
            FieldValue::from("{<<\"connection_name\">>,longstr,<<\"0E0E0EB0B020\">>}"),
            FieldValue::from("{<<\"copyright\">>,longstr,<<\"Galgus\">>}"),
            FieldValue::from("{<<\"hostname\">>,longstr,<<\"Galgus\">>}"),
            FieldValue::from("{<<\"product\">>,longstr,<<\"CHT\">>}"),
            FieldValue::from("{<<\"information\">>,longstr,<<\"https://www.galgus.net\">>}"),
            FieldValue::from("{<<\"version\">>,longstr,<<\"MockService\">>}"),
            FieldValue::from("{<<\"platform\">>,longstr,<<\"AX1800YCXT\">>}"),
            FieldValue::from("{<<\"ecoversion\">>,longstr,<<\"MockService\">>}"),
        ])
        .unwrap();
        let expected = FieldValue::A(f_array);
        let result: FieldValue = from_bytes(&input).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Frame {
            type_id: Octect,
            channel_id: ShortUint,
            size: LongUint,
            payload: LongStr,
            end: Octect,
        }
        let input = vec![
            0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x04, b'A', b'B', b'C',
            b'D', 0xCE,
        ];
        let expected = Frame {
            type_id: 1,
            channel_id: 2,
            size: 8,
            payload: "ABCD".try_into().unwrap(),
            end: 0xCE,
        };
        let result: Frame = from_bytes(&input).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test(FieldValue, FieldValue, FieldValue, FieldValue);

        let input = vec![b't', 0x01, b'u', 0x00, 0x09, b'f', 0x3F, 0xC0, 0, 0, b'V'];
        let result: Test = from_bytes(&input).unwrap();
        let expected = Test(
            FieldValue::t(true),
            FieldValue::u(9),
            FieldValue::f(1.5),
            FieldValue::V,
        );
        assert_eq!(expected, result);
    }

    #[test]
    fn test_map() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Frame {
            table: FieldTable,
        }

        fn create_field_table() -> FieldTable {
            let mut table = FieldTable::new();
            table.insert("A".try_into().unwrap(), FieldValue::t(true));
            table.insert("B".try_into().unwrap(), FieldValue::u(9));
            table.insert("C".try_into().unwrap(), FieldValue::f(1.5));
            table
        }

        let expected = Frame {
            table: create_field_table(),
        };

        let input = vec![
            0x00, 0x00, 0x00, 16, 0x01, b'A', b't', 0x01, 0x01, b'B', b'u', 0x00, 0x09, 0x01, b'C',
            b'f', 0x3F, 0xC0, 0, 0,
        ];
        let result: Frame = from_bytes(&input).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    #[should_panic(expected = "`Err` value: Incomplete")]
    fn test_incomplete_error() {
        #[derive(Deserialize)]
        struct Frame;
        let input = b"deadbeaf";

        let _: Frame = from_bytes(&input[..]).unwrap();
    }

    #[test]
    #[should_panic(expected = "`Err` value: Eof")]
    fn test_eof() {
        #[derive(Deserialize)]
        struct Frame(u32);
        let input = vec![0x00];
        let _: Frame = from_bytes(&input[..]).unwrap();
    }

    #[test]
    #[should_panic(expected = "`Err` value: ExpectedLength")]
    fn test_missing_length() {
        #[derive(Deserialize)]
        struct Frame(Vec<u8>);
        let input = vec![0, 1, 2];
        let _: Frame = from_bytes(&input[..]).unwrap();
    }

    #[test]
    #[should_panic(expected = "`Err` value: Syntax")]
    fn test_syntax_err() {
        #[derive(Deserialize)]
        struct Frame(u8, Vec<u8>);
        let input = vec![9, 0, 0];
        let _: Frame = from_bytes(&input[..]).unwrap();
    }

    #[test]
    fn test_deserialize_bytes_zero_copy() {
        #[derive(Deserialize)]
        struct Frame<'a> {
            _len: u8,
            m_bytes: &'a [u8], // require length field because `bytes` is variable lengh type,
        }

        let input = vec![
            0x04, b'b', b'e', b'e', b'f', // (4, b"beef")
        ];
        let result: Frame = from_bytes(&input).unwrap();

        assert_eq!(b"beef", result.m_bytes);
    }

    #[test]
    fn test_other_serde_data_models() {
        #[derive(Deserialize)]
        struct Frame {
            m_i8: i8,
            m_i16: i16,
            m_i32: i32,
            m_i64: i64,
            m_u64: u64,
            m_f64: f64,
            m_char: (u8, char), // require length field because `char` is variable lengh type,
            m_owned_bytes: (u8, Vec<u8>), // require length field because `Vec` is variable lengh type,
            m_opt: Option<u8>,
            m_unit: (),
        }

        let input = vec![
            0xff, // -1
            0xff, 0xfe, // -2
            0xff, 0xff, 0xff, 0xfd, // -3
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfc, // -4
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 9223372036854775808
            0x3F, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 1.5
            0x03, 0xE2, 0x82, 0xAC, // (3, '€')
            0x04, b'b', b'e', b'e', b'f', // (4, b"beef")
            b'o', // Some(b'o')
        ];
        let result: Frame = from_bytes(&input).unwrap();
        assert_eq!(-1, result.m_i8);
        assert_eq!(-2, result.m_i16);
        assert_eq!(-3, result.m_i32);
        assert_eq!(-4, result.m_i64);
        assert_eq!(9223372036854775808, result.m_u64);
        assert_eq!(1.5, result.m_f64);
        assert_eq!('€', result.m_char.1);
        assert_eq!(b"beef".to_vec(), result.m_owned_bytes.1);
        assert_eq!(Some(b'o'), result.m_opt);
        assert_eq!(0, std::mem::size_of_val(&result.m_unit));
    }
}
