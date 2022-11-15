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

pub fn to_buffer<T, U: BufMut + DerefMut<Target = [u8]>>(value: &T, buf: &mut U) -> Result<usize>
where
    T: Serialize,
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
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        let start = self.output.len();

        // reserve u32 for length of table
        self.serialize_u32(0)?;
        Ok(MapSerializer { ser: self, start })
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
        let len: u32 = (self.ser.output.len() - self.start - 4) as u32;

        let mut start = self.start;
        for b in len.to_be_bytes() {
            let p = self.ser.output.get_mut(start).unwrap();
            *p = b;
            start += 1;
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
    use serde::Serialize;

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
}
