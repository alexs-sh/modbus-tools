use crate::common;
use byteorder::{ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use std::cell::RefCell;
use std::io::Cursor;

pub trait Coils {
    /// write coils to a buffer
    /// return number of written coils
    fn coils_write(&self, dst: &mut [u8]) -> u16;

    /// return number of registers in a storage
    fn coils_count(&self) -> u16;
}

impl Coils for &[bool] {
    fn coils_write(&self, dst: &mut [u8]) -> u16 {
        let nbits = std::cmp::min(self.len(), dst.len() * 8) as u16;
        let len = common::ncoils_len(nbits);
        for (ibyte, byte) in dst.iter_mut().enumerate().take(len) {
            *byte = 0;
            for ibit in 0..8 {
                let idx = ibyte * 8 + ibit;
                if idx < nbits as usize {
                    let value = self[idx] as u8;
                    *byte |= value << ibit as u8;
                }
            }
        }
        nbits
    }

    fn coils_count(&self) -> u16 {
        self.len() as u16
    }
}

pub struct CoilsSlice<'a> {
    inner: RefCell<&'a [u8]>,
    nobjs: u16,
}

impl<'a> CoilsSlice<'a> {
    pub fn new(slice: &'a [u8], nobjs: u16) -> CoilsSlice<'a> {
        assert!(slice.remaining() >= common::ncoils_len(nobjs));
        CoilsSlice {
            inner: RefCell::new(slice),
            nobjs,
        }
    }
}

impl<'a> Coils for CoilsSlice<'a> {
    fn coils_write(&self, dst: &mut [u8]) -> u16 {
        let slen = common::ncoils_len(self.nobjs);
        let dlen = dst.len();
        assert!(dlen >= slen);
        let mut inner = self.inner.borrow_mut();
        inner.copy_to_slice(&mut dst[0..slen]);
        self.nobjs
    }

    fn coils_count(&self) -> u16 {
        self.nobjs
    }
}

pub struct CoilsCursor<'a, 'b> {
    inner: RefCell<&'a mut Cursor<&'b [u8]>>,
    nobjs: u16,
}

impl<'a, 'b> CoilsCursor<'a, 'b> {
    pub fn new(cursor: &'a mut Cursor<&'b [u8]>, nobjs: u16) -> CoilsCursor<'a, 'b> {
        assert!(cursor.remaining() >= common::ncoils_len(nobjs));
        CoilsCursor {
            inner: RefCell::new(cursor),
            nobjs,
        }
    }
}

impl<'a, 'b> Coils for CoilsCursor<'a, 'b> {
    fn coils_write(&self, dst: &mut [u8]) -> u16 {
        let slen = common::ncoils_len(self.nobjs);
        let dlen = dst.len();

        assert!(dlen >= slen);

        let mut dst = Cursor::new(dst);
        let mut inner = self.inner.borrow_mut();
        for _ in 0..slen {
            dst.write_u8(inner.read_u8().unwrap()).unwrap();
        }

        self.nobjs
    }

    fn coils_count(&self) -> u16 {
        self.nobjs
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn coils_slice() {
        let slice = CoilsSlice::new(&[0x0F], 4);
        let mut buffer = [0u8];
        assert_eq!(slice.coils_count(), 4);
        slice.coils_write(&mut buffer[..]);
        assert_eq!(buffer[0], 0x0F);
    }
}
