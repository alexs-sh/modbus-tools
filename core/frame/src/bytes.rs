use super::common;
use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use std::cell::RefCell;
use std::io::Cursor;

pub trait BytesStorage {
    /// write registers to a buffer
    /// return number of written registers
    fn bytes_write(&self, dst: &mut [u8]) -> u16;

    /// return number of registers in a storage
    fn bytes_count(&self) -> u16;
}

impl BytesStorage for &[u8] {
    fn bytes_write(&self, dst: &mut [u8]) -> u16 {
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

    fn bytes_count(&self) -> u16 {
        (self.len() / 2) as u16
    }
}

pub struct CursorBytes<'a, 'b> {
    inner: RefCell<&'a mut Cursor<&'b [u8]>>,
    nobjs: u16,
}

impl<'a, 'b> CursorBytes<'a, 'b> {
    pub fn new(cursor: &'a mut Cursor<&'b [u8]>, nobjs: u16) -> CursorBytes<'a, 'b> {
        assert!(cursor.remaining() >= nobjs as usize);
        CursorBytes {
            inner: RefCell::new(cursor),
            nobjs,
        }
    }
}

impl<'a, 'b> BytesStorage for CursorBytes<'a, 'b> {
    fn bytes_write(&self, dst: &mut [u8]) -> u16 {
        let slen = self.nobjs as usize;
        let dlen = dst.len();
        let len = std::cmp::min(slen, dlen);
        assert!(common::data_bytes_check(len as usize));

        let mut inner = self.inner.borrow_mut();
        for b in dst.iter_mut().take(len) {
            *b = inner.read_u8().unwrap();
        }

        len as u16
    }

    fn bytes_count(&self) -> u16 {
        self.nobjs
    }
}
