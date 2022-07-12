extern crate frame;

use crate::common::error::Error;
use frame::{header::Header, MAX_PDU_SIZE, MBAP_HEADER_LEN};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::{Buf, BytesMut};
use std::io::Cursor;

use tokio_util::codec::{Decoder, Encoder};

#[derive(Default)]
pub struct Codec;

impl Decoder for Codec {
    type Item = Header;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.remaining() < MBAP_HEADER_LEN {
            return Ok(None);
        }
        let mut src = Cursor::new(src.as_ref());
        let id = src.read_u16::<BigEndian>().unwrap();
        let proto = src.read_u16::<BigEndian>().unwrap();
        let len = src.read_u16::<BigEndian>().unwrap();
        let slave = src.read_u8().unwrap();

        if proto != 0 {
            Err(Error::InvalidVersion)
        } else if (len < 2) || (len as usize > (MAX_PDU_SIZE)) {
            Err(Error::InvalidData)
        } else {
            Ok(Some(Header {
                id,
                proto,
                len,
                slave,
            }))
        }
    }
}

impl Encoder<Header> for Codec {
    type Error = Error;
    fn encode(&mut self, header: Header, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let dst = &mut Cursor::new(dst.as_mut());
        dst.write_u16::<BigEndian>(header.id)?;
        dst.write_u16::<BigEndian>(0)?;
        dst.write_u16::<BigEndian>(header.len)?;
        dst.write_u8(header.slave)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_mbap_inv_proto() {
        let input = [0x00u8, 0x01, 0x00, 0x01, 0x00, 0x06, 0x11];
        let res = Codec::default().decode(&mut BytesMut::from(&input[..]));
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), Error::InvalidVersion);
    }

    #[test]
    fn read_mbap_inv_len() {
        let input = [0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x11];
        let res = Codec::default().decode(&mut BytesMut::from(&input[..]));
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), Error::InvalidData);
    }
}
