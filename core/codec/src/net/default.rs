extern crate frame;

use frame::{header::Header, MAX_PDU_SIZE};

use crate::common::{error::Error, packer, parser};
use bytes::{Buf, BytesMut};
use frame::{request::RequestFrame, response::ResponseFrame};
use tokio_util::codec::{Decoder, Encoder};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

use log::debug;

#[derive(Default)]
pub struct RequestDecoder {
    header: Option<Header>,
}

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
            decoder: RequestDecoder::default(),
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
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.log_bytes("unpack", src);
        self.decoder.decode(src)
    }
}

impl Encoder<ResponseFrame> for Codec {
    type Error = Error;
    fn encode(&mut self, msg: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let res = self.encoder.encode(msg, dst);
        self.log_bytes("pack", dst);
        res
    }
}

impl Decoder for RequestDecoder {
    type Item = RequestFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.header.is_none() && src.len() > 6 {
            let mut cursor = Cursor::new(src.as_ref());
            let header = parse_header(&mut cursor)?.unwrap();
            self.header = Some(header);
            src.advance(cursor.position() as usize);
        }

        let needed = self.header.as_ref().map_or(0, |header| header.len - 1) as usize;
        if needed > 0 && needed <= src.len() {
            let mut cursor = Cursor::new(&src.as_ref()[0..needed]);
            let func = cursor.read_u8().unwrap();
            let pdu = parser::parse_request(func, &mut cursor)?;
            if let Some(pdu) = pdu {
                src.advance(needed);
                let header = self.header.take().unwrap();
                let result = RequestFrame {
                    id: Some(header.id),
                    slave: header.slave,
                    pdu,
                };
                return Ok(Some(result));
            }
        }

        Ok(None)
    }
}

impl Encoder<ResponseFrame> for ResponseEncoder {
    type Error = Error;
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

pub(crate) fn parse_header(src: &mut Cursor<&[u8]>) -> Result<Option<Header>, Error> {
    if src.remaining() < 7 {
        return Ok(None);
    }

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

pub(crate) fn pack_header(header: &Header, dst: &mut Cursor<&mut [u8]>) -> Result<(), Error> {
    dst.write_u16::<BigEndian>(header.id)?;
    dst.write_u16::<BigEndian>(0)?;
    dst.write_u16::<BigEndian>(header.len)?;
    dst.write_u8(header.slave)?;
    Ok(())
}
#[cfg(test)]
mod test {
    use super::*;
    use frame::exception::Code;
    use frame::request::RequestPDU;
    use frame::response::{ResponseFrame, ResponsePDU};

    #[test]
    fn read_mbap_inv_proto() {
        let input = [0x00, 0x01, 0x00, 0x01, 0x00, 0x06, 0x11];
        let mut cursor = std::io::Cursor::new(&input[..]);
        let res = parse_header(&mut cursor);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), Error::InvalidVersion);
    }

    #[test]
    fn read_mbap_inv_len() {
        let input = [0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x11];
        let mut cursor = std::io::Cursor::new(&input[..]);
        let res = parse_header(&mut cursor);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), Error::InvalidData);
    }

    #[test]
    fn decode_fc3() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = RequestDecoder::default();
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
        let mut decoder = RequestDecoder::default();
        let message = decoder.decode(&mut bytes);
        assert!(message.is_err());
        assert_eq!(message.err().unwrap(), Error::InvalidVersion);
    }

    #[test]
    fn decode_fc3_part1() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = RequestDecoder::default();
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
        let mut decoder = RequestDecoder::default();
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

    #[test]
    fn decode_0x2b() {
        let input = [0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x11, 0x2B, 0x0E, 0x1];

        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = RequestDecoder::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id.unwrap(), 0x01);
        match message.pdu {
            RequestPDU::EncapsulatedInterfaceTransport { mei_type, data } => {
                assert_eq!(mei_type, 0xE);
                assert_eq!(data.get_u8(0).unwrap(), 0x1);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn encode_0x2b() {
        let control = [
            0x00, 0x03, 0x00, 0x00, 0x00, 0x06, 0x02, 0x2B, 0x0E, 0x31, 0x31, 0x31,
        ];
        let mut buffer = BytesMut::with_capacity(256);
        let mut encoder = ResponseEncoder::default();
        let pdu = ResponsePDU::encapsulated_interface_transport(0xE, "111".as_bytes());
        let frame = ResponseFrame::net(0x3, 0x2, pdu);
        let _ = encoder.encode(frame, &mut buffer).unwrap();
        assert_eq!(control, buffer.as_ref());
    }

    #[test]
    fn pack_exception() {
        let control = [0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x1, 0x83, 0x1];
        let buffer = [0u8; 256];
        let mut dst = BytesMut::from(&buffer[..]);
        let pdu = ResponsePDU::exception(0x3, Code::IllegalFunction);
        let frame = ResponseFrame::net(0x1, 0x1, pdu);
        let _ = ResponseEncoder::default().encode(frame, &mut dst).unwrap();
        assert_eq!(dst.len(), 9);
        assert_eq!(dst.as_ref(), control);
    }
}
