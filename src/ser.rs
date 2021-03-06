// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.

use std::result;

use collections::{Vec, String};

use byteorder::{ByteOrder, BigEndian, LittleEndian};

use serde;

use defs::*;
use error::*;

pub type Result = result::Result<(), Error>;

/// The corepack Serializer. Contains a closure that receives byte buffers as
/// the output is created.
pub struct Serializer<F: FnMut(&[u8]) -> Result> {
    output: F
}

impl<F: FnMut(&[u8]) -> Result> Serializer<F> {
    /// Create a new Serializer given an output function.
    pub const fn new(output: F) -> Serializer<F> {
        Serializer {
            output: output
        }
    }

    fn output(&mut self, buf: &[u8]) -> Result {
        self.output.call_mut((buf,))
    }
}

impl<F: FnMut(&[u8]) -> Result> serde::Serializer for Serializer<F> {
    type Error = Error;

    type SeqState = Option<(usize, Vec<u8>)>;
    type TupleState = Self::SeqState;
    type TupleStructState = Self::SeqState;
    type TupleVariantState = Self::TupleState;

    type MapState = Self::SeqState;
    type StructState = Self::MapState;
    type StructVariantState = Self::MapState;

    fn serialize_bool(&mut self, v: bool) -> Result {
        if v {
            self.output(&[TRUE])
        } else {
            self.output(&[FALSE])
        }
    }

    fn serialize_i64(&mut self, value: i64) -> Result {
        if value >= FIXINT_MIN as i64 && value <= FIXINT_MAX as i64 {
            let mut buf = [0; U16_BYTES];
            LittleEndian::write_i16(&mut buf, value as i16);
            self.output(&buf[..1])
        } else if value >= i8::min_value() as i64 && value <= i8::max_value() as i64 {
            let mut buf = [0; U16_BYTES];
            LittleEndian::write_i16(&mut buf, value as i16);
            self.output(&[INT8, buf[0]])
        } else if value >= 0 && value <= u8::max_value() as i64 {
            let mut buf = [0; U16_BYTES];
            LittleEndian::write_i16(&mut buf, value as i16);
            self.output(&[UINT8, buf[0]])
        } else if value >= i16::min_value() as i64 && value <= i16::max_value() as i64 {
            let mut buf = [INT16; U16_BYTES + 1];
            BigEndian::write_i16(&mut buf[1..], value as i16);
            self.output(&buf)
        } else if value >= 0 && value <= u16::max_value() as i64 {
            let mut buf = [UINT16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value as u16);
            self.output(&buf)
        } else if value >= i32::min_value() as i64 && value <= i32::max_value() as i64 {
            let mut buf = [INT32; U32_BYTES + 1];
            BigEndian::write_i32(&mut buf[1..], value as i32);
            self.output(&buf)
        } else if value >= 0 && value <= u32::max_value() as i64 {
            let mut buf = [UINT32; U16_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value as u32);
            self.output(&buf)
        } else {
            let mut buf = [INT64; U64_BYTES + 1];
            BigEndian::write_i64(&mut buf[1..], value);
            self.output(&buf)
        }
    }

    fn serialize_isize(&mut self, value: isize) -> Result {
        self.serialize_i64(value as i64)
    }

    fn serialize_i8(&mut self, value: i8) -> Result {
        self.serialize_i64(value as i64)
    }

    fn serialize_i16(&mut self, value: i16) -> Result {
        self.serialize_i64(value as i64)
    }

    fn serialize_i32(&mut self, value: i32) -> Result {
        self.serialize_i64(value as i64)
    }

    fn serialize_u64(&mut self, value: u64) -> Result {
        if value <= FIXINT_MAX as u64 {
            self.output(&[value as u8])
        } else if value <= u8::max_value() as u64 {
            self.output(&[UINT8, value as u8])
        } else if value <= u16::max_value() as u64 {
            let mut buf = [UINT16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value as u16);
            self.output(&buf)
        } else if value <= u32::max_value() as u64 {
            let mut buf = [UINT32; U32_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value as u32);
            self.output(&buf)
        } else {
            let mut buf = [UINT64; U64_BYTES + 1];
            BigEndian::write_u64(&mut buf[1..], value);
            self.output(&buf)
        }
    }

    fn serialize_usize(&mut self, value: usize) -> Result {
        self.serialize_u64(value as u64)
    }

    fn serialize_u8(&mut self, value: u8) -> Result {
        self.serialize_u64(value as u64)
    }

    fn serialize_u16(&mut self, value: u16) -> Result {
        self.serialize_u64(value as u64)
    }

    fn serialize_u32(&mut self, value: u32) -> Result {
        self.serialize_u64(value as u64)
    }

    fn serialize_f32(&mut self, value: f32) -> Result {
        let mut buf = [FLOAT32; U32_BYTES + 1];
        BigEndian::write_f32(&mut buf[1..], value);
        self.output(&buf)
    }

    fn serialize_f64(&mut self, value: f64) -> Result {
        let mut buf = [FLOAT64; U64_BYTES + 1];
        BigEndian::write_f64(&mut buf[1..], value);
        self.output(&buf)
    }

    fn serialize_str(&mut self, value: &str) -> Result {
        if value.len() <= MAX_FIXSTR {
            try!(self.output(&[value.len() as u8 | FIXSTR_MASK]));
        } else if value.len() <= MAX_STR8 {
            try!(self.output(&[STR8, value.len() as u8]));
        } else if value.len() <= MAX_STR16 {
            let mut buf = [STR16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value.len() as u16);
            try!(self.output(&buf));
        } else if value.len() <= MAX_STR32 {
            let mut buf = [STR32; U32_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value.len() as u32);
            try!(self.output(&buf));
        } else {
            return Err(Error::simple(Reason::TooBig));
        }

        self.output(value.as_bytes())
    }

    fn serialize_char(&mut self, v: char) -> Result {
        let mut string = String::new();
        string.push(v);

        self.serialize_str(&*string)
    }

    fn serialize_unit(&mut self) -> Result {
        self.output(&[NIL])
    }

    fn serialize_unit_struct(&mut self, _: &'static str) -> Result {
        self.serialize_unit()
    }

    fn serialize_unit_variant(&mut self, _: &'static str, index: usize, _: &'static str) -> Result {
        self.serialize_usize(index)
    }

    fn serialize_newtype_struct<T>(&mut self, name: &'static str, value: T) -> Result
        where T: serde::Serialize {
        let mut state = try!(self.serialize_tuple_struct(name, 1));
        try!(self.serialize_tuple_struct_elt(&mut state, value));
        self.serialize_tuple_struct_end(state)
    }

    fn serialize_newtype_variant<T>(&mut self, name: &'static str, variant_index: usize, variant: &'static str, value: T) -> Result
        where T: serde::Serialize {
        let mut state = try!(self.serialize_tuple_variant(name, variant_index, variant, 1));
        try!(self.serialize_tuple_variant_elt(&mut state, value));
        self.serialize_tuple_variant_end(state)
    }

    fn serialize_none(&mut self) -> Result {
        self.serialize_unit()
    }

    fn serialize_some<V>(&mut self, value: V) -> Result
        where V: serde::Serialize {
        value.serialize(self)
    }

    fn serialize_seq(&mut self, len: Option<usize>) -> result::Result<Self::SeqState, Error> {
        if let Some(size) = len {
            // output the size now

            if size <= MAX_FIXARRAY {
                try!(self.output(&[size as u8 | FIXARRAY_MASK]));
            } else if size <= MAX_ARRAY16 {
                let mut buf = [ARRAY16; U16_BYTES + 1];
                BigEndian::write_u16(&mut buf[1..], size as u16);
                try!(self.output(&buf));
            } else if size <= MAX_ARRAY32 {
                let mut buf = [ARRAY32; U32_BYTES + 1];
                BigEndian::write_u32(&mut buf[1..], size as u32);
                try!(self.output(&buf));
            } else {
                return Err(Error::simple(Reason::TooBig));
            }

            // No state needed
            Ok(None)
        } else {
            Ok(Some((0, vec![])))
        }
    }

    fn serialize_seq_fixed_size(&mut self, size: usize) -> result::Result<Self::SeqState, Error> {
        self.serialize_seq(Some(size))
    }

    fn serialize_seq_elt<T>(&mut self, state: &mut Self::SeqState, value: T) -> Result
        where T: serde::Serialize {
        if let &mut Some((ref mut size, ref mut buffer)) = state {
            let mut target = Serializer::new(move |bytes| {
                buffer.extend_from_slice(bytes);
                Ok(())
            });

            *size += 1;

            value.serialize(&mut target)
        } else {
            value.serialize(self)
        }
    }

    fn serialize_seq_end(&mut self, state: Self::SeqState) -> Result {
        if let Some((size, buffer)) = state {
            if size <= MAX_FIXARRAY {
                try!(self.output(&[size as u8 | FIXARRAY_MASK]));
            } else if size <= MAX_ARRAY16 {
                let mut buf = [ARRAY16; U16_BYTES + 1];
                BigEndian::write_u16(&mut buf[1..], size as u16);
                try!(self.output(&buf));
            } else if size <= MAX_ARRAY32 {
                let mut buf = [ARRAY32; U32_BYTES + 1];
                BigEndian::write_u32(&mut buf[1..], size as u32);
                try!(self.output(&buf));
            } else {
                return Err(Error::simple(Reason::TooBig));
            }

            self.output(buffer.as_slice())
        } else {
            Ok(())
        }
    }

    fn serialize_tuple(&mut self, len: usize) -> result::Result<Self::SeqState, Error> {
        self.serialize_seq_fixed_size(len)
    }

    fn serialize_tuple_elt<T>(&mut self, state: &mut Self::SeqState, value: T) -> Result
        where T: serde::Serialize {
        self.serialize_seq_elt(state, value)
    }

    fn serialize_tuple_end(&mut self, state: Self::SeqState) -> Result {
        self.serialize_seq_end(state)
    }

    fn serialize_tuple_struct(&mut self, _: &'static str, len: usize) -> result::Result<Self::SeqState, Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_struct_elt<T>(&mut self, state: &mut Self::SeqState, value: T) -> Result
        where T: serde::Serialize {
        self.serialize_tuple_elt(state, value)
    }

    fn serialize_tuple_struct_end(&mut self, state: Self::SeqState) -> Result {
        self.serialize_tuple_end(state)
    }

    fn serialize_tuple_variant(&mut self, _: &'static str, index: usize, _: &'static str, len: usize) -> result::Result<Self::TupleVariantState, Error> {
        let mut state = try!(self.serialize_tuple(len + 1));
        // serialize the variant index as an extra element at the front
        try!(self.serialize_tuple_elt(&mut state, index));

        Ok(state)
    }

    fn serialize_tuple_variant_elt<T>(&mut self, state: &mut Self::SeqState, value: T) -> Result
        where T: serde::Serialize {
        self.serialize_tuple_elt(state, value)
    }

    fn serialize_tuple_variant_end(&mut self, state: Self::SeqState) -> Result {
        self.serialize_tuple_end(state)
    }

    fn serialize_map(&mut self, len: Option<usize>) -> result::Result<Self::MapState, Error> {
        if let Some(size) = len {
            if size <= MAX_FIXMAP {
                try!(self.output(&[size as u8 | FIXMAP_MASK]));
            } else if size <= MAX_MAP16 {
                let mut buf = [MAP16; U16_BYTES + 1];
                BigEndian::write_u16(&mut buf[1..], size as u16);
                try!(self.output(&buf));
            } else if size <= MAX_MAP32 {
                let mut buf = [MAP32; U32_BYTES + 1];
                BigEndian::write_u32(&mut buf[1..], size as u32);
                try!(self.output(&buf));
            } else {
                return Err(Error::simple(Reason::TooBig));
            }

            Ok(None)
        } else {
            Ok(Some((0, vec![])))
        }
    }

    fn serialize_map_key<T>(&mut self, state: &mut Self::MapState, key: T) -> Result
        where T: serde::Serialize {
        self.serialize_seq_elt(state, key)
    }

    fn serialize_map_value<T>(&mut self, state: &mut Self::MapState, value: T) -> Result
        where T: serde::Serialize {
        self.serialize_seq_elt(state, value)
    }

    fn serialize_map_end(&mut self, state: Self::MapState) -> Result {
        if let Some((size, buffer)) = state {
            if size <= MAX_FIXMAP {
                try!(self.output(&[size as u8 | FIXMAP_MASK]));
            } else if size <= MAX_MAP16 {
                let mut buf = [MAP16; U16_BYTES + 1];
                BigEndian::write_u16(&mut buf[1..], size as u16);
                try!(self.output(&buf));
            } else if size <= MAX_MAP32 {
                let mut buf = [MAP32; U32_BYTES + 1];
                BigEndian::write_u32(&mut buf[1..], size as u32);
                try!(self.output(&buf));
            } else {
                return Err(Error::simple(Reason::TooBig));
            }

            self.output(buffer.as_slice())
        } else {
            Ok(())
        }
    }

    fn serialize_struct(&mut self, _: &'static str, len: usize) -> result::Result<Self::MapState, Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_elt<V>(&mut self, state: &mut Self::MapState, key: &'static str, value: V) -> Result
        where V: serde::Serialize {
        try!(self.serialize_map_key(state, key));
        self.serialize_map_value(state, value)
    }

    fn serialize_struct_end(&mut self, state: Self::MapState) -> Result {
        self.serialize_map_end(state)
    }

    fn serialize_struct_variant(&mut self, name: &'static str, index: usize, _: &'static str, len: usize) -> result::Result<Self::MapState, Error> {
        // encode a struct variant as a tuple of the variant index plus the struct itself
        let mut state = try!(self.serialize_tuple(2));

        // state in this case should statically be None, so only check in debug builds
        debug_assert!(state.is_none(), "Tuple state was not None");

        // that means we can just throw recreate it later

        try!(self.serialize_tuple_elt(&mut state, index));

        // messagepack uses pascal-style arrays for objects, so we can just keep encoding things
        // and get the same result as if we called serialize_elt. This is a bit of a hack, though.

        self.serialize_struct(name, len)
    }

    fn serialize_struct_variant_elt<V>(&mut self, state: &mut Self::MapState, key: &'static str, value: V) -> Result
        where V: serde::Serialize {
        self.serialize_struct_elt(state, key, value)
    }

    fn serialize_struct_variant_end(&mut self, state: Self::MapState) -> Result {
        try!(self.serialize_struct_end(state));

        // end the tuple here as well, re-creating the state
        // we asserted earlier that the state in this case should be None, since this
        // is a fixed-sized sequence
        self.serialize_tuple_end(None)
    }

    fn serialize_bytes(&mut self, value: &[u8]) -> Result {
        if value.len() <= MAX_BIN8 {
            try!(self.output(&[BIN8, value.len() as u8]));
        } else if value.len() <= MAX_BIN16 {
            let mut buf = [BIN16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value.len() as u16);
            try!(self.output(&buf));
        } else if value.len() <= MAX_BIN32 {
            let mut buf = [BIN32; U32_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value.len() as u32);
            try!(self.output(&buf));
        } else {
            return Err(Error::simple(Reason::TooBig));
        }

        self.output(value)
    }
}

#[cfg(test)]
mod test {
    use collections::{Vec, String};
    use collections::btree_map::BTreeMap;

    #[test]
    fn positive_fixint_test() {
        let v: u8 = 23;
        assert_eq!(::to_bytes(v).unwrap(), &[0x17]);
    }
    #[test]
    fn negative_fixint_test() {
        let v: i8 = -5;
        assert_eq!(::to_bytes(v).unwrap(), &[0xfb]);
    }

    #[test]
    fn uint8_test() {
        let v: u8 = 154;
        assert_eq!(::to_bytes(v).unwrap(), &[0xcc, 0x9a]);
    }

    #[test]
    fn fixstr_test() {
        let s: &str = "Hello World!";
        assert_eq!(::to_bytes(s).unwrap(), &[0xac, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
                                             0x57, 0x6f, 0x72, 0x6c, 0x64, 0x21]);
    }

    #[test]
    fn str8_test() {
        let s: &str = "The quick brown fox jumps over the lazy dog";
        let mut fixture: Vec<u8> = vec![];
        fixture.push(0xd9);
        fixture.push(s.len() as u8);
        fixture.extend_from_slice(s.as_bytes());
        assert_eq!(::to_bytes(s).unwrap(), fixture);
    }

    #[test]
    fn fixarr_test() {
        let v: Vec<u8> = vec![5, 8, 20, 231];
        assert_eq!(::to_bytes(v).unwrap(), &[0x94, 0x05, 0x08, 0x14, 0xcc, 0xe7]);
    }

    #[test]
    fn array16_test() {
        let v: Vec<isize> = vec![-5, 16, 101, -45, 184,
                                 89, 62, -233, -33, 304,
                                 76, 90, 23, 108, 45,
                                 -3, 2];
        assert_eq!(::to_bytes(v).unwrap(), &[0xdc,
                                             0x00, 0x11,
                                             0xfb,  0x10,  0x65,  0xd0, 0xd3,  0xcc, 0xb8,
                                             0x59,  0x3e,  0xd1, 0xff, 0x17,  0xd0, 0xdf,  0xd1, 0x01, 0x30,
                                             0x4c, 0x5a, 0x17, 0x6c, 0x2d,
                                             0xfd, 0x02]);
    }

    #[test]
    fn fixmap_test() {
        let mut map: BTreeMap<String, usize> = BTreeMap::new();
        map.insert("one".into(), 1);
        map.insert("two".into(), 2);
        map.insert("three".into(), 3);
        assert_eq!(::to_bytes(map).unwrap(), &[0x83,
                                               0xa3, 0x6f, 0x6e, 0x65,  0x01,
                                               0xa5, 0x74, 0x68, 0x72, 0x65, 0x65,  0x03,
                                               0xa3, 0x74, 0x77, 0x6f,  0x02]);
    }
}
