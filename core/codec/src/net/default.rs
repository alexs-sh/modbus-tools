extern crate frame;

use frame::{header::Header, MBAP_HEADER_LEN};

use crate::common::{
    error::Error, header::Codec as CodecHeader, request::Codec as CodecPDURequest,
    response::Codec as CodecPDUResponse,
};
use bytes::{Buf, BytesMut};
use frame::{request::RequestFrame, response::ResponseFrame};
use tokio_util::codec::{Decoder, Encoder};

use log::debug;

pub struct Codec {
    header: Option<Header>,
    name: String,
}

impl Codec {
    pub fn new(name: String) -> Codec {
        Codec { name, header: None }
    }

    fn log_bytes(&self, prefix: &'static str, bytes: &mut BytesMut) {
        if !bytes.is_empty() {
            debug!("{} {} {:?}", self.name, prefix, bytes.as_ref());
        }
    }
}

impl Default for Codec {
    fn default() -> Codec {
        Codec::new("NetCodec".to_owned())
    }
}

impl Decoder for Codec {
    type Item = RequestFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.log_bytes("unpack", src);
        if self.header.is_none() && src.len() >= MBAP_HEADER_LEN {
            let header = CodecHeader::default().decode(src)?.unwrap();
            self.header = Some(header);
            src.advance(MBAP_HEADER_LEN);
        }

        let needed = self.header.as_ref().map_or(0, |header| header.len - 1) as usize;
        let read_pdu = needed > 0 && needed <= src.len();
        let pdu = if read_pdu {
            CodecPDURequest::default().decode(src)?.map(|pdu| {
                src.advance(needed);
                RequestFrame::net_parts(self.header.take().unwrap(), pdu)
            })
        } else {
            None
        };

        Ok(pdu)
    }
}

impl Encoder<ResponseFrame> for Codec {
    type Error = Error;
    fn encode(&mut self, msg: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload_size = msg.pdu.len() + 1;
        let full_size = 6 + payload_size;
        dst.resize(full_size, 0);

        let header = Header::new(msg.id.unwrap(), payload_size as u16, msg.slave);
        CodecHeader::default().encode(header, dst)?;

        let mut body = dst.split_off(7);
        CodecPDUResponse::default().encode(msg.pdu, &mut body)?;

        dst.unsplit(body);

        self.log_bytes("pack", dst);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use frame::exception::Code;
    use frame::request::RequestPDU;
    use frame::response::{ResponseFrame, ResponsePDU};

    #[test]
    fn decode_fc3() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = Codec::default();
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
        let mut decoder = Codec::default();
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
        let mut decoder = Codec::default();
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
        let mut decoder = Codec::default();
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
        let mut decoder = Codec::default();
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
        let mut encoder = Codec::default();
        let pdu = ResponsePDU::encapsulated_interface_transport(0xE, "111".as_bytes());
        let frame = ResponseFrame::net(0x3, 0x2, pdu);
        let _ = encoder.encode(frame, &mut buffer).unwrap();
        assert_eq!(control, buffer.as_ref());
    }

    #[test]
    fn encode_exception() {
        let control = [0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x1, 0x83, 0x1];
        let buffer = [0u8; 256];
        let mut dst = BytesMut::from(&buffer[..]);
        let pdu = ResponsePDU::exception(0x3, Code::IllegalFunction);
        let frame = ResponseFrame::net(0x1, 0x1, pdu);
        let _ = Codec::default().encode(frame, &mut dst).unwrap();
        assert_eq!(dst.len(), 9);
        assert_eq!(dst.as_ref(), control);
    }
}
