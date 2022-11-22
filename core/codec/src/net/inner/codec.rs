extern crate frame;
use super::header::{Header, HeaderCodec};
use crate::{error::Error, pdu::PduRequestCodec, pdu::PduResponseCodec};
use bytes::{Buf, BytesMut};
use frame::{RequestFrame, ResponseFrame, MBAP_HEADER_LEN};
use log::debug;
use tokio_util::codec::{Decoder, Encoder};

pub struct NetCodec {
    header: Option<Header>,
    name: String,
}

impl NetCodec {
    pub fn new(name: &str) -> NetCodec {
        NetCodec {
            name: name.to_owned(),
            header: None,
        }
    }

    fn log_bytes(&self, prefix: &'static str, bytes: &mut BytesMut) {
        if !bytes.is_empty() {
            debug!("{} {} {:?}", self.name, prefix, bytes.as_ref());
        }
    }
}

impl Default for NetCodec {
    fn default() -> NetCodec {
        NetCodec::new("NetCodec")
    }
}

impl Decoder for NetCodec {
    type Item = RequestFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.log_bytes("unpack", src);
        if self.header.is_none() && src.len() >= MBAP_HEADER_LEN {
            let header = HeaderCodec::default().decode(src)?.unwrap();
            self.header = Some(header);
            src.advance(MBAP_HEADER_LEN);
        }

        let needed = self.header.as_ref().map_or(0, |header| header.len - 1) as usize;
        let read_pdu = needed > 0 && needed <= src.len();
        let request = if read_pdu {
            PduRequestCodec::default().decode(src)?.map(|pdu| {
                src.advance(needed);
                let header = self.header.take().unwrap();
                RequestFrame::from_parts(header.id, header.slave, pdu)
            })
        } else {
            None
        };

        Ok(request)
    }
}

impl Encoder<ResponseFrame> for NetCodec {
    type Error = Error;
    fn encode(&mut self, msg: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload_size = msg.pdu.len() + 1;
        let full_size = 6 + payload_size;
        dst.resize(full_size, 0);

        let header = Header::new(msg.id, payload_size as u16, msg.slave);
        HeaderCodec::default().encode(header, dst)?;

        let mut body = dst.split_off(7);
        PduResponseCodec::default().encode(msg.pdu, &mut body)?;

        dst.unsplit(body);

        self.log_bytes("pack", dst);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::Error;
    use frame::exception::Code;
    use frame::{RequestPdu, ResponsePdu};

    #[test]
    fn decode_fc1() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x01, 0x00, 0x01, 0x00, 0xA,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x01);
        match message.pdu {
            RequestPdu::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x1);
                assert_eq!(nobjs, 0xA);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_fc2() {
        let input = [
            0x00, 0x02, 0x00, 0x00, 0x00, 0x06, 0x12, 0x02, 0x00, 0x03, 0x00, 0xB,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x12);
        assert_eq!(message.id, 0x02);
        match message.pdu {
            RequestPdu::ReadDiscreteInputs { address, nobjs } => {
                assert_eq!(address, 0x3);
                assert_eq!(nobjs, 0xB);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_fc3() {
        let input = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x01);
        match message.pdu {
            RequestPdu::ReadHoldingRegisters { address, nobjs } => {
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
        let mut decoder = NetCodec::default();
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
        let mut decoder = NetCodec::default();
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
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x01);
        match message.pdu {
            RequestPdu::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x6B);
                assert_eq!(nobjs, 0x3);
            }
            _ => unreachable!(),
        };

        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x12);
        assert_eq!(message.id, 0x02);
        match message.pdu {
            RequestPdu::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x7B);
                assert_eq!(nobjs, 0x3);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn decode_fc4() {
        let input = [
            0x00, 0x04, 0x00, 0x00, 0x00, 0x06, 0x14, 0x04, 0x00, 0xA, 0x00, 0xF,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x14);
        assert_eq!(message.id, 0x04);
        match message.pdu {
            RequestPdu::ReadInputRegisters { address, nobjs } => {
                assert_eq!(address, 0xA);
                assert_eq!(nobjs, 0xF);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_fc5() {
        let input = [
            0x00, 0x04, 0x00, 0x00, 0x00, 0x06, 0x11, 0x05, 0x00, 0xAC, 0xFF, 0x00,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x04);
        match message.pdu {
            RequestPdu::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0xAC);
                assert_eq!(value, true);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_fc5_inv_value() {
        let input = [
            0x00, 0x04, 0x00, 0x00, 0x00, 0x06, 0x11, 0x05, 0x00, 0xAC, 0x00, 0x01,
        ];
        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes);
        assert!(message.is_err());
        assert_eq!(message.err().unwrap(), Error::InvalidData);
    }

    #[test]
    fn decode_fc6() {
        let input = [
            0x00, 0x04, 0x00, 0x00, 0x00, 0x06, 0x11, 0x06, 0x00, 0xAD, 0x13, 0x13,
        ];

        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x04);
        match message.pdu {
            RequestPdu::WriteSingleRegister { address, value } => {
                assert_eq!(address, 0xAD);
                assert_eq!(value, 0x1313);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_fc15() {
        let input = [
            0x00, 0x05, 0x00, 0x00, 0x00, 0x06, 0x11, 0x0F, 0x00, 0x13, 0x00, 0x0A, 0x02, 0xCD,
            0x01,
        ];

        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x05);
        match message.pdu {
            RequestPdu::WriteMultipleCoils {
                address,
                nobjs,
                data,
            } => {
                assert_eq!(address, 0x13);
                assert_eq!(nobjs, 0x0A);
                assert_eq!(data.len(), 2);
                assert_eq!(data.get_u8(0).unwrap(), 0xCD);
                assert_eq!(data.get_u8(1).unwrap(), 0x01);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_fc16() {
        let input = [
            0x00, 0x06, 0x00, 0x00, 0x00, 0x06, 0x11, 0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00,
            0x0A, 0x01, 0x02,
        ];

        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x06);
        match message.pdu {
            RequestPdu::WriteMultipleRegisters {
                address,
                nobjs,
                data,
            } => {
                assert_eq!(address, 0x1);
                assert_eq!(nobjs, 0x02);
                assert_eq!(data.len(), 4);
                assert_eq!(data.get_u16(0).unwrap(), 0x000A);
                assert_eq!(data.get_u16(1).unwrap(), 0x0102);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_0x2b() {
        let input = [0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x11, 0x2B, 0x0E, 0x1];

        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = NetCodec::default();
        let message = decoder.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(message.slave, 0x11);
        assert_eq!(message.id, 0x01);
        match message.pdu {
            RequestPdu::EncapsulatedInterfaceTransport { mei_type, data } => {
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
        let mut encoder = NetCodec::default();
        let response = ResponseFrame::from_parts(
            0x03,
            0x02,
            ResponsePdu::encapsulated_interface_transport(0xE, "111".as_bytes()),
        );
        let _ = encoder.encode(response, &mut buffer).unwrap();
        assert_eq!(control, buffer.as_ref());
    }

    #[test]
    fn encode_exception() {
        let control = [0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x1, 0x83, 0x1];
        let buffer = [0u8; 256];
        let mut dst = BytesMut::from(&buffer[..]);
        let response =
            ResponseFrame::from_parts(0x1, 0x1, ResponsePdu::exception(0x3, Code::IllegalFunction));
        let _ = NetCodec::default().encode(response, &mut dst).unwrap();
        assert_eq!(dst.len(), 9);
        assert_eq!(dst.as_ref(), control);
    }
}
