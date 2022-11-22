extern crate frame;
use crate::error::Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::{Buf, BytesMut};
use frame::{MAX_DATA_SIZE, MAX_PDU_SIZE, MBAP_HEADER_LEN};
use std::io::Cursor;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug)]
pub struct Header {
    pub id: u16,
    pub proto: u16,
    pub len: u16,
    pub slave: u8,
}

impl Header {
    pub fn new(id: u16, len: u16, slave: u8) -> Header {
        assert!(len > 0);
        assert!(len < MAX_DATA_SIZE as u16);
        Header {
            id,
            proto: 0,
            len,
            slave,
        }
    }
}
#[derive(Default)]
pub struct HeaderCodec;

impl Decoder for HeaderCodec {
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

impl Encoder<Header> for HeaderCodec {
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
        let res = HeaderCodec::default().decode(&mut BytesMut::from(&input[..]));
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), Error::InvalidVersion);
    }

    #[test]
    fn read_mbap_inv_len() {
        let input = [0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x11];
        let res = HeaderCodec::default().decode(&mut BytesMut::from(&input[..]));
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), Error::InvalidData);
    }

    #[test]
    fn write_header() {
        let control = [0x00u8, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11];
        let header = Header::new(0x1, 0x6, 0x11);
        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        HeaderCodec::default().encode(header, &mut buffer).unwrap();
        assert_eq!(&control[..], &buffer[..]);
    }
}
