extern crate frame;

use frame::{header::Header, MAX_PDU_SIZE};

use crate::common::{error::CodecError, packer, parser};
use bytes::{Buf, BytesMut};
use frame::{request::RequestFrame, response::ResponseFrame};
use tokio_util::codec::{Decoder, Encoder};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

use log::debug;

#[derive(Default)]
pub struct RequestDecoder;

#[derive(Default)]
pub struct ResponseEncoder;

pub struct Codec {
    decoder: RequestDecoder,
    encoder: ResponseEncoder,
    name: String,
}

impl Codec {
    pub fn new(name: String) -> Codec {
        Codec {
            decoder: RequestDecoder,
            encoder: ResponseEncoder,
            name,
        }
    }

    fn log_bytes(&self, prefix: &'static str, bytes: &mut BytesMut) {
        if !bytes.is_empty() {
            debug!("{} {} {:?}", self.name, prefix, bytes.as_ref());
        }
    }
}

impl Decoder for Codec {
    type Item = RequestFrame;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.log_bytes("unpack", src);
        self.decoder.decode(src)
    }
}

impl Encoder<ResponseFrame> for Codec {
    type Error = CodecError;
    fn encode(&mut self, msg: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let res = self.encoder.encode(msg, dst);
        self.log_bytes("pack", dst);
        res
    }
}

impl Decoder for RequestDecoder {
    type Item = RequestFrame;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 8 {
            return Ok(None);
        }

        let mut cursor = Cursor::new(src.as_ref());

        let header = parse_header(&mut cursor)?.unwrap();
        let func = cursor.read_u8().unwrap();
        let pdu = parser::parse_request(func, &mut cursor)?;
        let pos = cursor.position() as usize;
        pdu.map_or(Ok(None), |pdu| {
            src.advance(pos);
            Ok(Some(RequestFrame {
                id: Some(header.id),
                slave: header.slave,
                pdu,
            }))
        })
    }
}

impl Encoder<ResponseFrame> for ResponseEncoder {
    type Error = CodecError;
    fn encode(&mut self, msg: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload_size = msg.pdu.len() + 1;
        let full_size = 6 + payload_size;
        dst.resize(full_size, 0);

        let mut cursor = Cursor::new(dst.as_mut());
        let header = Header::new(msg.id.unwrap(), payload_size as u16, msg.slave);
        pack_header(&header, &mut cursor)?;
        packer::pack_response(&msg.pdu, &mut cursor)?;
        Ok(())
    }
}

pub(crate) fn parse_header(src: &mut Cursor<&[u8]>) -> Result<Option<Header>, CodecError> {
    if src.remaining() < 7 {
        return Ok(None);
    }

    let id = src.read_u16::<BigEndian>().unwrap();
    let proto = src.read_u16::<BigEndian>().unwrap();
    let len = src.read_u16::<BigEndian>().unwrap();
    let slave = src.read_u8().unwrap();

    if proto != 0 {
        Err(CodecError::InvalidVersion)
    } else if (len < 2) || (len as usize > (MAX_PDU_SIZE)) {
        Err(CodecError::InvalidData)
    } else {
        Ok(Some(Header {
            id,
            proto,
            len,
            slave,
        }))
    }
}

pub(crate) fn pack_header(header: &Header, dst: &mut Cursor<&mut [u8]>) -> Result<(), CodecError> {
    dst.write_u16::<BigEndian>(header.id)?;
    dst.write_u16::<BigEndian>(0)?;
    dst.write_u16::<BigEndian>(header.len)?;
    dst.write_u8(header.slave)?;
    Ok(())
}
#[cfg(test)]
mod test {
    use super::*;
    use frame::request::RequestPDU;

    #[test]
    fn read_mbap_inv_proto() {
        let input = [0x00, 0x01, 0x00, 0x01, 0x00, 0x06, 0x11];
        let mut cursor = std::io::Cursor::new(&input[..]);
        let res = parse_header(&mut cursor);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), CodecError::InvalidVersion);
    }

    #[test]
    fn read_mbap_inv_len() {
        let input = [0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x11];
        let mut cursor = std::io::Cursor::new(&input[..]);
        let res = parse_header(&mut cursor);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), CodecError::InvalidData);
    }

    #[test]
    fn decode_fc3() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = RequestDecoder {};
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id.unwrap(), 0x01);
        match message.pdu {
            RequestPDU::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x6B);
                assert_eq!(nobjs, 0x3);
            }
            _ => unreachable!(),
        }
    }
    #[test]
    fn decode_fc3_inv1() {
        let input = [
            0x00, 0x01, 0x00, 0x01, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = RequestDecoder {};
        let message = decoder.decode(&mut bytes);
        assert!(message.is_err());
        assert_eq!(message.err().unwrap(), CodecError::InvalidVersion);
    }

    #[test]
    fn decode_fc3_part1() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = RequestDecoder {};
        let message = decoder.decode(&mut bytes);
        assert!(message.is_ok());
        assert_eq!(message.unwrap(), None);
    }

    #[test]
    fn decode_fc3_twice() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x06, 0x12, 0x03, 0x00, 0x7B, 0x00, 0x03,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = RequestDecoder {};
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id.unwrap(), 0x01);
        match message.pdu {
            RequestPDU::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x6B);
                assert_eq!(nobjs, 0x3);
            }
            _ => unreachable!(),
        };

        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x12);
        assert_eq!(message.id.unwrap(), 0x02);
        match message.pdu {
            RequestPDU::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x7B);
                assert_eq!(nobjs, 0x3);
            }
            _ => unreachable!(),
        };
    }
}
