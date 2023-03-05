use byteorder::{BigEndian, NativeEndian, ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use std::io::Cursor;

pub(crate) struct ReadCtx<'a> {
    pub buffer: &'a [u8],
    pub cursor: Cursor<&'a [u8]>,
}

impl<'a> ReadCtx<'a> {
    pub fn new(buffer: &'a [u8]) -> ReadCtx {
        ReadCtx {
            buffer,
            cursor: Cursor::new(buffer),
        }
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        self.cursor.read_u8().ok()
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        self.cursor.read_u16::<NativeEndian>().ok()
    }

    pub fn read_u16_be(&mut self) -> Option<u16> {
        self.cursor.read_u16::<BigEndian>().ok()
    }

    pub fn remaining(&self) -> usize {
        self.cursor.remaining()
    }

    pub fn processed(&self) -> usize {
        self.cursor.position() as usize
    }

    pub fn is_enough(&self, size: usize) -> Option<bool> {
        if self.remaining() >= size {
            Some(true)
        } else {
            None
        }
    }
}

pub(crate) struct WriteCtx<'a> {
    pub cursor: Cursor<&'a mut [u8]>,
}

impl<'a> WriteCtx<'a> {
    pub fn new(buffer: &'a mut [u8]) -> WriteCtx {
        WriteCtx {
            //       buffer,
            cursor: Cursor::new(buffer),
        }
    }

    pub fn write_u8(&mut self, value: u8) -> Option<()> {
        self.cursor.write_u8(value).ok()
    }

    pub fn write_u16(&mut self, value: u16) -> Option<()> {
        self.cursor.write_u16::<NativeEndian>(value).ok()
    }

    pub fn write_u16_be(&mut self, value: u16) -> Option<()> {
        self.cursor.write_u16::<BigEndian>(value).ok()
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Option<()> {
        for byte in bytes {
            if self.cursor.write_u8(*byte).is_err() {
                return None;
            }
        }
        Some(())
    }

    pub fn write_data_u16_be(&mut self, values: &[u8]) -> Option<()> {
        assert!(values.len() % 2 == 0);
        let len = values.len() / 2;
        for idx in 0..len {
            let b1 = values[idx * 2];
            let b2 = values[idx * 2 + 1];
            let value = u16::from_ne_bytes([b1, b2]);
            if self.cursor.write_u16::<BigEndian>(value).is_err() {
                return None;
            }
        }
        Some(())
    }

    pub fn remaining(&self) -> usize {
        self.cursor.remaining()
    }

    pub fn processed(&self) -> usize {
        self.cursor.position() as usize
    }

    pub fn buffer(&self) -> &[u8] {
        self.cursor.get_ref()
    }

    pub fn is_enough(&self, size: usize) -> Option<bool> {
        if self.remaining() >= size {
            Some(true)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ReadCtx, WriteCtx};

    #[test]
    fn read_ctx() {
        let buffer = [0x00, 0x01];
        let mut ctx = ReadCtx::new(&buffer);
        assert_eq!(ctx.processed(), 0);
        assert_eq!(ctx.remaining(), 2);
        ctx.read_u8().unwrap();
        assert_eq!(ctx.processed(), 1);
        assert_eq!(ctx.remaining(), 1);
        ctx.read_u8().unwrap();
        assert_eq!(ctx.processed(), 2);
        assert_eq!(ctx.remaining(), 0);
        assert!(ctx.read_u8().is_none());
    }

    #[test]
    fn read_ctx_und() {
        let buffer = [0x01];
        let mut ctx = ReadCtx::new(&buffer);
        let res = ctx.read_u16();
        assert!(res.is_none());
    }

    #[test]
    fn write_ctx() {
        let mut buffer = [0u8; 2];
        let mut ctx = WriteCtx::new(&mut buffer);
        assert_eq!(ctx.processed(), 0);
        assert_eq!(ctx.remaining(), 2);
        ctx.write_u8(1).unwrap();
        assert_eq!(ctx.processed(), 1);
        assert_eq!(ctx.remaining(), 1);
        ctx.write_u8(2);
        assert_eq!(ctx.processed(), 2);
        assert_eq!(ctx.remaining(), 0);
        assert!(ctx.write_u8(3).is_none());
        assert_eq!(buffer, [0x1, 0x2]);
    }
}
