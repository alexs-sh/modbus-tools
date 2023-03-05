use crate::data::helpers;
use byteorder::{BigEndian, NativeEndian, ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use std::cell::RefCell;
use std::io::Cursor;

pub trait Registers {
    /// write registers to a buffer
    /// return number of written registers
    fn registers_write(&self, dst: &mut [u8]) -> u16;

    /// return number of registers in a storage
    fn registers_count(&self) -> u16;
}

impl Registers for &[u8] {
    fn registers_write(&self, dst: &mut [u8]) -> u16 {
        let slen = self.len();
        let dlen = dst.len();
        let len = (std::cmp::min(slen, dlen) / 2) as u16;
        let mut src = Cursor::new(self);
        let mut dst = Cursor::new(dst);

        for _ in 0..len {
            dst.write_u16::<NativeEndian>(src.read_u16::<NativeEndian>().unwrap())
                .unwrap();
        }

        len
    }

    fn registers_count(&self) -> u16 {
        (self.len() / 2) as u16
    }
}

impl Registers for &[u16] {
    fn registers_write(&self, dst: &mut [u8]) -> u16 {
        let slen = self.len() * 2;
        let dlen = dst.len();
        let len = (std::cmp::min(slen, dlen) / 2) as u16;
        let mut dst = Cursor::new(dst);

        for i in 0..len as usize {
            dst.write_u16::<NativeEndian>(self[i]).unwrap();
        }

        len
    }

    fn registers_count(&self) -> u16 {
        self.len() as u16
    }
}

pub struct RegistersCursorBe<'a, 'b> {
    inner: RefCell<&'a mut Cursor<&'b [u8]>>,
    nobjs: u16,
}

impl<'a, 'b> RegistersCursorBe<'a, 'b> {
    pub fn new(cursor: &'a mut Cursor<&'b [u8]>, nobjs: u16) -> RegistersCursorBe<'a, 'b> {
        assert!(cursor.remaining() >= helpers::get_registers_len(nobjs));
        RegistersCursorBe {
            inner: RefCell::new(cursor),
            nobjs,
        }
    }
}

impl<'a, 'b> Registers for RegistersCursorBe<'a, 'b> {
    fn registers_write(&self, dst: &mut [u8]) -> u16 {
        let slen = helpers::get_registers_len(self.nobjs);
        let dlen = dst.len();
        let nobj = (std::cmp::min(slen, dlen) / 2) as u16;
        let mut dst = Cursor::new(dst);
        let mut inner = self.inner.borrow_mut();
        for _ in 0..nobj {
            dst.write_u16::<BigEndian>(inner.read_u16::<NativeEndian>().unwrap())
                .unwrap();
        }

        nobj
    }

    fn registers_count(&self) -> u16 {
        self.nobjs
    }
}

#[cfg(test)]

mod test {
    use super::*;

    #[test]
    fn with_u8() {
        let input = [1u8, 2, 3, 4, 5, 6];
        let mut output = [0u8; 6];
        let rs = &input[..];
        let res = rs.registers_write(&mut output[..]);
        assert_eq!(res, 3);
        assert_eq!(rs.registers_count(), 3);
        assert_eq!(&input, &output);
    }
}
