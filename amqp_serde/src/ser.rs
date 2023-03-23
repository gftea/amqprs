use crate::error::{Error, Result};
use bytes::BufMut;
use serde::{ser, Serialize};
use std::ops::DerefMut;

pub struct Serializer<'a, W: BufMut> {
    output: &'a mut W, // TODO: buffer generic interfaces?
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut buf = Vec::new();
    let mut serializer = Serializer { output: &mut buf };
    value.serialize(&mut serializer)?;
    Ok(buf)
}

pub fn to_buffer<T, U>(value: &T, buf: &mut U) -> Result<usize>
where
    T: Serialize,
    U: BufMut + DerefMut<Target = [u8]>,
{
    let initial_size = buf.len();
    let mut serializer = Serializer { output: buf };
    value.serialize(&mut serializer)?;
    let final_size = buf.len();
    Ok(final_size - initial_size)
}

impl<'a, 'b: 'a, W> ser::Serializer for &'a mut Serializer<'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = MapSerializer<'a, 'b, W>;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.output.put_u8(v as u8);

        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.output.put_u8(v as u8);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.output.put_u8(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.output.put(&v.to_be_bytes()[..]);

        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.output.put(v.as_bytes());

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.output.put(v);

        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    // Do nothing
    fn serialize_unit(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        variant.serialize(&mut *self)
    }

    // serialize only contained value
    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    // serialize variant name and contained value
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)
    }

    // ignore length
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    // same as seq
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    // same as seq
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        variant.serialize(&mut *self)?;
        self.serialize_seq(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        variant.serialize(&mut *self)?;
        self.serialize_seq(Some(len))
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_seq(Some(len))
    }

    // map is mainly for AMQP field-table, implicitly serailize length as `u32`
    // if to skip serailizing length, one can implement `Serialize` by passing `None` to `len`
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let start = self.output.len();
        if len.is_none() {
            // reserve u32 for length of table
            self.serialize_u32(0)?;
        }
        Ok(MapSerializer { ser: self, start, is_len_known: len.is_some() })
    }
}

impl<'a, 'b: 'a, W> ser::SerializeSeq for &'a mut Serializer<'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}
impl<'a, 'b: 'a, W> ser::SerializeTuple for &'a mut Serializer<'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}
impl<'a, 'b: 'a, W> ser::SerializeTupleStruct for &'a mut Serializer<'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}
impl<'a, 'b: 'a, W> ser::SerializeTupleVariant for &'a mut Serializer<'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

pub struct MapSerializer<'a, 'b: 'a, W: BufMut> {
    ser: &'a mut Serializer<'b, W>,
    start: usize,
    is_len_known: bool,
}

impl<'a, 'b: 'a, W> ser::SerializeMap for MapSerializer<'a, 'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();
    type Error = Error;

    // The Serde data model allows map keys to be any serializable type.
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        // first 4 bytes are reserved for length
        if !self.is_len_known {
            let len: u32 = (self.ser.output.len() - self.start - 4) as u32;

            let mut start = self.start;
            for b in len.to_be_bytes() {
                let p = self.ser.output.get_mut(start).unwrap();
                *p = b;
                start += 1;
            }
        }
        Ok(())
    }
}

impl<'a, 'b: 'a, W> ser::SerializeStruct for &'a mut Serializer<'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'b: 'a, W> ser::SerializeStructVariant for &'a mut Serializer<'b, W>
where
    W: BufMut + DerefMut<Target = [u8]>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}
/////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod test {
    use crate::to_bytes;
    use crate::types::*;
    use serde::{Serialize, Serializer, ser::SerializeMap};
    use std::collections::BTreeMap;

    #[test]
    fn test_size() {
        println!("{:?}", std::mem::size_of_val(&String::from("s")));
        println!("{:?}", std::mem::size_of_val("s"));
        println!("{:?}", std::mem::size_of_val(&FieldValue::t(true)));
    }

    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct Frame {
            type_id: Octect,
            channel_id: ShortUint,
            size: LongUint,
            payload: LongStr,
        }

        impl Frame {
            fn new() -> Self {
                Frame {
                    type_id: 1,
                    channel_id: 2,
                    size: 3,
                    payload: "ABCD".try_into().unwrap(),
                }
            }
        }

        let test = Frame::new();
        let expected = vec![
            0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, b'A', b'B', b'C',
            b'D',
        ];
        assert_eq!(to_bytes(&test).unwrap(), expected);
    }

    #[test]
    fn test_field_table() {
        fn create_field_table() -> FieldTable {
            let mut table = FieldTable::new();
            table.insert("A".try_into().unwrap(), FieldValue::t(true));
            table.insert("B".try_into().unwrap(), FieldValue::u(9));
            table.insert("C".try_into().unwrap(), FieldValue::f(1.5));
            table.insert("D".try_into().unwrap(), FieldValue::V);

            table
        }

        let test = create_field_table();
        let result = to_bytes(&test).unwrap();

        // HashMap is not ordered according to insert
        // search the field name first, then get the field-value-pair

        // length bytes
        let len_expected = vec![0x00, 0x00, 0x00, 19];
        assert_eq!(len_expected, result[..4]);

        // 'A' field-value pair
        let a_expected = vec![0x01, b'A', b't', 0x01];
        let a = result.iter().position(|v| v == &b'A').unwrap();
        assert_eq!(a_expected, result[a - 1..a + 3]);

        // 'B' field-value pair
        let b_expected = vec![0x01, b'B', b'u', 0x00, 0x09];
        let b = result.iter().position(|v| v == &b'B').unwrap();
        assert_eq!(b_expected, result[b - 1..b + 4]);

        // 'C' field-value pair
        let c_expected = vec![0x01, b'C', b'f', 0x3F, 0xC0, 0, 0];
        let c = result.iter().position(|v| v == &b'C').unwrap();
        assert_eq!(c_expected, result[c - 1..c + 6]);

        // 'D' field-value pair
        let d_expected = vec![0x01, b'D', b'V'];
        let d = result.iter().position(|v| v == &b'D').unwrap();
        assert_eq!(d_expected, result[d - 1..d + 2]);

        // total number of bytes
        // println!("{:02X?}", result);
        assert_eq!(
            len_expected.len()
                + a_expected.len()
                + b_expected.len()
                + c_expected.len()
                + d_expected.len(),
            result.len()
        );
    }

    #[test]
    fn test_other_serde_data_models() {
        #[derive(Serialize)]
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
        let frame = Frame {
            m_i8: -1,
            m_i16: -2,
            m_i32: -3,
            m_i64: -4,
            m_u64: 0x80_00_00_00_00_00_00_00,
            m_f64: 1.5,
            m_char: (3, '€'),
            m_owned_bytes: (4, b"beef".to_vec()),
            m_opt: Some(b'o'),
            m_unit: (),
        };
        let expected = vec![
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
        let result = to_bytes(&frame).unwrap();
        assert_eq!(expected, result);
    }


    #[test]
    fn test_serialize_map_known_length_up_front() {
        // We use BTreeMap in order to garantee that it iterates in a sorted way
        struct TestStruct<K, V>(BTreeMap<K, V>);

        impl<K, V> Serialize for TestStruct<K, V>
        where
            K: Serialize + AsRef<[u8]>,
            V: Serialize + AsRef<[u8]>,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                /*
                 * In this case the Serialize impl for the map has to manage by itself
                * the serialization of the map lenght
                 */
                let len = self.0
                    .iter()
                    .fold(0, |l, (k, v)| { l + (k.as_ref().len() + v.as_ref().len()) as u32 });
                let mut map = serializer.serialize_map(Some(self.0.len()))?; // Known up-front length
                map.serialize_value(&len)?;
                for (k, v) in self.0.iter() {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }

        let mut map = BTreeMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        map.insert("key3", "value3");
        map.insert("key4", "value4");
        let ts = TestStruct(map);

        let result = to_bytes(&ts).unwrap();
        let expected = vec![
            0x00, 0x00, 0x00, 0x28, // len = 4 entries
            b'k', b'e', b'y', b'1', b'v', b'a', b'l', b'u', b'e', b'1', // first entry
            b'k', b'e', b'y', b'2', b'v', b'a', b'l', b'u', b'e', b'2', // sencond entry
            b'k', b'e', b'y', b'3', b'v', b'a', b'l', b'u', b'e', b'3', // thrid entry
            b'k', b'e', b'y', b'4', b'v', b'a', b'l', b'u', b'e', b'4', // fourth entry
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_serialize_map_unknown_length_up_front() {
        // We use BTreeMap in order to garantee that it iterates in a sorted way
        struct TestStruct<K, V>(BTreeMap<K, V>);

        impl<K, V> Serialize for TestStruct<K, V>
        where
            K: Serialize,
            V: Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let mut map = serializer.serialize_map(None)?; // Unknown up-front length
                for (k, v) in self.0.iter() {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }

        let mut map = BTreeMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        map.insert("key3", "value3");
        map.insert("key4", "value4");
        let ts = TestStruct(map);

        let result = to_bytes(&ts).unwrap();
        let expected = vec![
            0x00, 0x00, 0x00, 0x28, // len (10 bytes * 4 entries) = 40 bytes
            b'k', b'e', b'y', b'1', b'v', b'a', b'l', b'u', b'e', b'1', // first entry
            b'k', b'e', b'y', b'2', b'v', b'a', b'l', b'u', b'e', b'2', // sencond entry
            b'k', b'e', b'y', b'3', b'v', b'a', b'l', b'u', b'e', b'3', // thrid entry
            b'k', b'e', b'y', b'4', b'v', b'a', b'l', b'u', b'e', b'4', // fourth entry
        ];
        assert_eq!(expected, result);
    }
}
