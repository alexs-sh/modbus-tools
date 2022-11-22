extern crate frame;
use crate::error::Error;
use crate::net::inner::codec::NetCodec;
use bytes::BytesMut;
use frame::{RequestFrame, ResponseFrame};
use log::error;
use tokio_util::codec::{Decoder, Encoder};

pub struct UdpCodec {
    codec: NetCodec,
}

impl UdpCodec {
    pub fn new(name: &str) -> UdpCodec {
        UdpCodec {
            codec: NetCodec::new(name),
        }
    }
}

impl Default for UdpCodec {
    fn default() -> UdpCodec {
        UdpCodec::new("UdpCodec")
    }
}

impl Decoder for UdpCodec {
    type Item = RequestFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Ignore the errors, because we don't create new streams for UDP clients
        let res = self.codec.decode(src).map_or_else(
            |err| {
                error!("parser error:{:?}", err);
                Ok(None)
            },
            Ok,
        );
        src.clear();
        res
    }
}

impl Encoder<ResponseFrame> for UdpCodec {
    type Error = Error;
    fn encode(&mut self, msg: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.codec.encode(msg, dst)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn decode_error() {
        let input = [
            0x00, 0x06, 0x00, 0x00, 0x00, 0x06, 0x11, 0x10, 0x00, 0x01, 0x00, 0x02, 0x00, 0x0A,
            0x01, 0x02,
        ];

        let mut bytes = BytesMut::from(&input[..]);
        let mut decoder = UdpCodec::default();
        let message = decoder.decode(&mut bytes);
        assert!(message.is_ok());
        assert_eq!(message.unwrap(), None);
    }
}
