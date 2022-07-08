use super::common;
use byteorder::{BigEndian, NativeEndian, ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use std::cell::RefCell;
use std::io::Cursor;

pub trait RegisterStorage {
    /// write registers to a buffer
    /// return number of written registers
    fn registers_write(&self, dst: &mut [u8]) -> u16;

    /// return number of registers in a storage
    fn registers_count(&self) -> u16;
}

impl RegisterStorage for &[u8] {
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

impl RegisterStorage for &[u16] {
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

pub struct CursorBe<'a, 'b> {
    inner: RefCell<&'a mut Cursor<&'b [u8]>>,
    nobjs: u16,
}

impl<'a, 'b> CursorBe<'a, 'b> {
    pub fn new(cursor: &'a mut Cursor<&'b [u8]>, nobjs: u16) -> CursorBe<'a, 'b> {
        assert!(cursor.remaining() >= common::nregs_len(nobjs));
        CursorBe {
            inner: RefCell::new(cursor),
            nobjs,
        }
    }
}

impl<'a, 'b> RegisterStorage for CursorBe<'a, 'b> {
    fn registers_write(&self, dst: &mut [u8]) -> u16 {
        let slen = common::nregs_len(self.nobjs as u16);
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
