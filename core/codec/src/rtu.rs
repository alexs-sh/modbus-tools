//extern crate protocol;
//use crate::common::error::CodecError;
//use protocol::frame::request::RequestFrame;
//use bytes::BytesMut;
//use tokio_util::codec::Decoder;
//
//pub(crate) struct RequestDecoder;
//pub(crate) struct ResponseEncoder;
//
//impl Decoder for RequestDecoder {
//    type Item = RequestFrame;
//    type Error = CodecError;
//
//    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
//        let len = src.len();
//
//        // At least slave and func
//        if len < 2 {
//            return Ok(None);
//        }
//
//        //let slave = src[0];
//        //let func = src[1];
//
//        //let address = NetworkEndian::read_u16(&src[2..4]);
//
//        Ok(None)
//    }
//}
