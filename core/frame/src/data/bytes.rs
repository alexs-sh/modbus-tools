use crate::common;
use byteorder::ReadBytesExt;
use bytes::Buf;
use std::cell::RefCell;
use std::io::Cursor;

pub trait Bytes {
    /// write registers to a buffer
    /// return number of written registers
    fn bytes_write(&self, dst: &mut [u8]) -> u16;

    /// return number of registers in a storage
    fn bytes_count(&self) -> u16;
}

impl Bytes for &[u8] {
    fn bytes_write(&self, dst: &mut [u8]) -> u16 {
        let len = std::cmp::min(self.len(), dst.len());
        dst[..len].copy_from_slice(&self[..len]);
        len as u16
    }

    fn bytes_count(&self) -> u16 {
        self.len() as u16
    }
}

pub struct BytesCursor<'a, 'b> {
    inner: RefCell<&'a mut Cursor<&'b [u8]>>,
    nobjs: u16,
}

impl<'a, 'b> BytesCursor<'a, 'b> {
    pub fn new(cursor: &'a mut Cursor<&'b [u8]>, nobjs: u16) -> BytesCursor<'a, 'b> {
        assert!(cursor.remaining() >= nobjs as usize);
        BytesCursor {
            inner: RefCell::new(cursor),
            nobjs,
        }
    }
}

impl<'a, 'b> Bytes for BytesCursor<'a, 'b> {
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

#[cfg(test)]

mod test {
    use super::*;

    #[test]
    fn test_with_u8() {
        let input = [1u8, 2, 3, 4];
        let mut output = [0u8; 4];
        let bs: &dyn Bytes = &input.as_slice();
        assert_eq!(bs.bytes_count(), 4);
        let res = bs.bytes_write(&mut output[..]);
        assert_eq!(res, 4);
        assert_eq!(input, output);
    }

    #[test]
    fn test_with_cursor() {
        let input = [1u8, 2, 3, 4];
        let mut output = [0u8; 4];
        let mut cursor = Cursor::new(&input[..]);
        let bs = BytesCursor::new(&mut cursor, 4);
        assert_eq!(bs.bytes_count(), 4);
        let res = bs.bytes_write(&mut output[..]);
        assert_eq!(res, 4);
        assert_eq!(input, output);
    }
}
