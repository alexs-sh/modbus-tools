use crate::{error::Error, helpers, pdu::PduRequestCodec, pdu::PduResponseCodec};
use bytes::{Buf, BytesMut};
use frame::{RequestFrame, RequestPdu, ResponseFrame};

use byteorder::{NativeEndian, WriteBytesExt};
use std::io::Cursor;
use tokio_util::codec::{Decoder, Encoder};

pub struct RtuCodec {
    slave: Option<u8>,
    request: Option<RequestPdu>,
    crc: u16,
    name: String,
}

impl Default for RtuCodec {
    fn default() -> RtuCodec {
        RtuCodec::new("serial")
    }
}

impl RtuCodec {
    pub fn new(name: &str) -> RtuCodec {
        RtuCodec {
            slave: None,
            request: None,
            crc: 0x0,
            name: name.to_owned(),
        }
    }

    pub fn reset(&mut self) {
        self.slave = None;
        self.request = None;
        self.crc = 0;
    }

    pub fn in_progress(&self) -> bool {
        self.slave.is_some()
    }

    fn update_crc(&mut self, bytes: &[u8]) -> u16 {
        self.crc = calc_crc_inner(self.crc, bytes);
        self.get_crc()
    }

    fn start_crc(&mut self) {
        self.crc = 0xFFFF;
    }

    fn get_crc(&self) -> u16 {
        u16::from_be(self.crc)
    }

    fn encode_slave(&mut self, slave: u8, dst: &mut BytesMut) -> Result<(), Error> {
        let dst = &mut Cursor::new(dst.as_mut());
        dst.write_u8(slave)?;
        Ok(())
    }

    fn encode_crc(&mut self, crc: u16, dst: &mut BytesMut) -> Result<(), Error> {
        let dst = &mut Cursor::new(dst.as_mut());
        dst.write_u16::<NativeEndian>(crc)?;
        Ok(())
    }

    fn decode_slave(&mut self, src: &mut BytesMut) -> Result<Option<RequestFrame>, Error> {
        if self.slave.is_none() && !src.is_empty() {
            let slave = src[0];
            self.slave = Some(slave);
            self.update_crc(&[slave]);
            src.advance(1);
        }
        Ok(None)
    }

    fn decode_pdu(&mut self, src: &mut BytesMut) -> Result<Option<RequestFrame>, Error> {
        if self.slave.is_some() && self.request.is_none() {
            if let Some(pdu) = PduRequestCodec::default().decode(src)? {
                self.update_crc(&src.as_ref()[..pdu.len()]);
                src.advance(pdu.len());
                self.request = Some(pdu);
            }
        }
        Ok(None)
    }

    fn decode_crc(&mut self, src: &mut BytesMut) -> Result<Option<RequestFrame>, Error> {
        if self.slave.is_some() && self.request.is_some() && src.len() >= 2 {
            let result = if self.update_crc(&src.as_ref()[..2]) == 0 {
                let request =
                    RequestFrame::new(self.slave.take().unwrap(), self.request.take().unwrap());
                Ok(Some(request))
            } else {
                Err(Error::InvalidData)
            };

            src.advance(2);
            result
        } else {
            Ok(None)
        }
    }
}

impl Decoder for RtuCodec {
    type Item = RequestFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        helpers::log_data(&self.name, "in", src);

        if self.slave.is_none() {
            self.start_crc();
        }

        let result = self
            .decode_slave(src)
            .and_then(|_| self.decode_pdu(src))
            .and_then(|_| self.decode_crc(src));

        match result {
            Ok(None) => {}
            Err(_) => {
                self.reset();
                src.clear();
            }
            _ => {
                self.reset();
            }
        }

        result
    }
}

impl Encoder<ResponseFrame> for RtuCodec {
    type Error = Error;
    fn encode(&mut self, msg: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let pdu_len = msg.pdu.len();
        let full_len = pdu_len + 3;
        dst.resize(full_len, 0);

        let mut crc = dst.split_off(full_len - 2);
        let mut body = dst.split_off(1);
        let mut head = dst.split_off(0);
        let result = self
            .encode_slave(msg.slave, &mut head)
            .and_then(|_| PduResponseCodec::default().encode(msg.pdu, &mut body))
            .and_then(|_| {
                let mut crc_val = 0xFFFF;
                crc_val = calc_crc_inner(crc_val, &head);
                crc_val = calc_crc_inner(crc_val, &body);
                self.encode_crc(crc_val, &mut crc)
            });

        self.reset();
        dst.unsplit(head);
        dst.unsplit(body);
        dst.unsplit(crc);

        helpers::log_data(&self.name, "out", dst);

        result
    }
}
const CRC16: [u16; 256] = [
    0x0000u16, 0xc0c1, 0xc181, 0x0140, 0xc301, 0x03c0, 0x0280, 0xc241, 0xc601, 0x06c0, 0x0780,
    0xc741, 0x0500, 0xc5c1, 0xc481, 0x0440, 0xcc01, 0x0cc0, 0x0d80, 0xcd41, 0x0f00, 0xcfc1, 0xce81,
    0x0e40, 0x0a00, 0xcac1, 0xcb81, 0x0b40, 0xc901, 0x09c0, 0x0880, 0xc841, 0xd801, 0x18c0, 0x1980,
    0xd941, 0x1b00, 0xdbc1, 0xda81, 0x1a40, 0x1e00, 0xdec1, 0xdf81, 0x1f40, 0xdd01, 0x1dc0, 0x1c80,
    0xdc41, 0x1400, 0xd4c1, 0xd581, 0x1540, 0xd701, 0x17c0, 0x1680, 0xd641, 0xd201, 0x12c0, 0x1380,
    0xd341, 0x1100, 0xd1c1, 0xd081, 0x1040, 0xf001, 0x30c0, 0x3180, 0xf141, 0x3300, 0xf3c1, 0xf281,
    0x3240, 0x3600, 0xf6c1, 0xf781, 0x3740, 0xf501, 0x35c0, 0x3480, 0xf441, 0x3c00, 0xfcc1, 0xfd81,
    0x3d40, 0xff01, 0x3fc0, 0x3e80, 0xfe41, 0xfa01, 0x3ac0, 0x3b80, 0xfb41, 0x3900, 0xf9c1, 0xf881,
    0x3840, 0x2800, 0xe8c1, 0xe981, 0x2940, 0xeb01, 0x2bc0, 0x2a80, 0xea41, 0xee01, 0x2ec0, 0x2f80,
    0xef41, 0x2d00, 0xedc1, 0xec81, 0x2c40, 0xe401, 0x24c0, 0x2580, 0xe541, 0x2700, 0xe7c1, 0xe681,
    0x2640, 0x2200, 0xe2c1, 0xe381, 0x2340, 0xe101, 0x21c0, 0x2080, 0xe041, 0xa001, 0x60c0, 0x6180,
    0xa141, 0x6300, 0xa3c1, 0xa281, 0x6240, 0x6600, 0xa6c1, 0xa781, 0x6740, 0xa501, 0x65c0, 0x6480,
    0xa441, 0x6c00, 0xacc1, 0xad81, 0x6d40, 0xaf01, 0x6fc0, 0x6e80, 0xae41, 0xaa01, 0x6ac0, 0x6b80,
    0xab41, 0x6900, 0xa9c1, 0xa881, 0x6840, 0x7800, 0xb8c1, 0xb981, 0x7940, 0xbb01, 0x7bc0, 0x7a80,
    0xba41, 0xbe01, 0x7ec0, 0x7f80, 0xbf41, 0x7d00, 0xbdc1, 0xbc81, 0x7c40, 0xb401, 0x74c0, 0x7580,
    0xb541, 0x7700, 0xb7c1, 0xb681, 0x7640, 0x7200, 0xb2c1, 0xb381, 0x7340, 0xb101, 0x71c0, 0x7080,
    0xb041, 0x5000, 0x90c1, 0x9181, 0x5140, 0x9301, 0x53c0, 0x5280, 0x9241, 0x9601, 0x56c0, 0x5780,
    0x9741, 0x5500, 0x95c1, 0x9481, 0x5440, 0x9c01, 0x5cc0, 0x5d80, 0x9d41, 0x5f00, 0x9fc1, 0x9e81,
    0x5e40, 0x5a00, 0x9ac1, 0x9b81, 0x5b40, 0x9901, 0x59c0, 0x5880, 0x9841, 0x8801, 0x48c0, 0x4980,
    0x8941, 0x4b00, 0x8bc1, 0x8a81, 0x4a40, 0x4e00, 0x8ec1, 0x8f81, 0x4f40, 0x8d01, 0x4dc0, 0x4c80,
    0x8c41, 0x4400, 0x84c1, 0x8581, 0x4540, 0x8701, 0x47c0, 0x4680, 0x8641, 0x8201, 0x42c0, 0x4380,
    0x8341, 0x4100, 0x81c1, 0x8081, 0x4040,
];

fn calc_crc(bytes: &[u8]) -> u16 {
    let crc = calc_crc_inner(0xFFFF, bytes);
    u16::from_be(crc)
}

fn calc_crc_inner(crc: u16, bytes: &[u8]) -> u16 {
    let mut new_crc = crc;
    for byte in bytes {
        let idx = ((new_crc ^ (*byte as u16)) & 0xFF) as usize;
        new_crc = new_crc >> 8 ^ CRC16[idx];
    }
    new_crc
}

#[cfg(test)]
mod test {
    use super::calc_crc;
    use super::ResponseFrame;
    use super::RtuCodec;
    use bytes::{Buf, BytesMut};
    use frame::data::coils::CoilsSlice;
    use frame::{RequestPdu, ResponsePdu};
    use tokio_util::codec::{Decoder, Encoder};
    #[test]
    fn crc_values() {
        let input = [
            (vec![0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25 /*0E84*/], 0x0E84),
            (vec![0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x84], 0x0),
            (vec![0x11, 0x04, 0x00, 0x08, 0x00, 0x01, 0xB2, 0x98], 0x0),
        ];

        for (data, crc) in input {
            assert_eq!(calc_crc(data.as_ref()), crc);
        }
    }

    #[test]
    fn crc_values_codec() {
        let input = [
            (vec![0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25 /*0E84*/], 0x0E84),
            (vec![0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x84], 0x0),
            (vec![0x11, 0x04, 0x00, 0x08, 0x00, 0x01, 0xB2, 0x98], 0x0),
        ];

        for (data, crc) in input {
            let mut codec = RtuCodec::default();
            codec.start_crc();
            codec.update_crc(data.as_ref());
            assert_eq!(calc_crc(data.as_ref()), crc);
        }
    }

    #[test]
    fn crc_values_codec_step() {
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25 /*0E84*/];
        let mut codec = RtuCodec::default();
        codec.start_crc();
        for b in input {
            codec.update_crc(&[b]);
        }
        assert_eq!(codec.get_crc(), 0x0E84);
    }

    #[test]
    fn decode_fc1() {
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x84];
        let mut buffer = BytesMut::from(&input[..]);
        let mut codec = RtuCodec::default();
        let msg = codec.decode(&mut buffer).unwrap().unwrap();
        match msg.pdu {
            RequestPdu::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x13);
                assert_eq!(nobjs, 0x25);
            }
            _ => unimplemented!(),
        }
        assert_eq!(buffer.len(), 0);
    }
    #[test]
    fn decode_fc1_crc_err() {
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x1E, 0x84];
        let mut buffer = BytesMut::from(&input[..]);
        let mut codec = RtuCodec::default();
        let msg = codec.decode(&mut buffer);
        match msg {
            Err(_) => {}
            _ => unimplemented!(),
        }
        assert_eq!(buffer.len(), 0);
    }
    #[test]
    fn decode_fc1_part() {
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E];
        let mut buffer = BytesMut::from(&input[..]);
        let mut codec = RtuCodec::default();
        let msg = codec.decode(&mut buffer).unwrap();
        match msg {
            None => (),
            _ => unimplemented!(),
        }
        assert_eq!(buffer.len(), 1);
    }
    #[test]
    fn decode_fc1_2x() {
        let input = [
            0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x84, 0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25,
            0x0E, 0x84,
        ];
        let mut buffer = BytesMut::from(&input[..]);
        let mut codec = RtuCodec::default();
        for i in 0..2 {
            let msg = codec.decode(&mut buffer).unwrap().unwrap();
            match msg.pdu {
                RequestPdu::ReadCoils { address, nobjs } => {
                    assert_eq!(address, 0x13);
                    assert_eq!(nobjs, 0x25);
                }
                _ => unimplemented!(),
            }
            assert_eq!(buffer.len(), 16 - (i + 1) * 8);
        }
    }
    #[test]
    fn encode_fc1() {
        let control = [0x11u8, 0x01, 0x05, 0xCD, 0x6B, 0xB2, 0x0E, 0x1B, 0x45, 0xE6];
        let mut buffer = BytesMut::with_capacity(512);
        let mut codec = RtuCodec::default();
        let msg = ResponseFrame::new(
            0x11,
            ResponsePdu::read_coils(CoilsSlice::new(&[0xCDu8, 0x6B, 0xB2, 0x0E, 0x1B], 37)),
        );
        codec.encode(msg, &mut buffer).unwrap();
        assert_eq!(10, buffer.chunk().len());
        assert_eq!(control, buffer.chunk());
    }
}
