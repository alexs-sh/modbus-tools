use crate::codec::context::{ReadCtx, WriteCtx};
use crate::codec::error::Error;
use crate::codec::mbap::{read_mbap, write_mbap};
use crate::codec::pduext::{read_pdu, write_pdu};
use crate::codec::rtuext::calc_crc_be;
use crate::codec::wait;

use crate::frame::prelude::*;
use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

fn read_u8(ctx: &mut ReadCtx) -> Result<Option<u8>, Error> {
    Ok(ctx.read_u8())
}

fn write_u8(ctx: &mut WriteCtx, value: u8) -> Result<Option<u8>, Error> {
    ctx.write_u8(value).unwrap();
    Ok(Some(value))
}

fn resize_buffer(dst: &mut BytesMut, size: usize) {
    dst.resize(size, 0);
}

fn read_crc(ctx: &mut ReadCtx) -> Result<Option<u16>, Error> {
    let crc = wait!(ctx.read_u16_be());
    let end = ctx.processed();
    let calc = calc_crc_be(&ctx.buffer[..end]);
    if calc == 0 {
        Ok(Some(crc))
    } else {
        Err(Error::InvalidCrc)
    }
}

fn write_crc(ctx: &mut WriteCtx) -> Result<Option<u16>, Error> {
    let data = &ctx.buffer()[..ctx.processed()];
    let crc = calc_crc_be(data);
    ctx.write_u16_be(crc).unwrap();
    Ok(Some(crc))
}

fn read_rtu_frame(ctx: &mut ReadCtx) -> Result<Option<RequestFrame>, Error> {
    let slave = wait!(read_u8(ctx)?); // else { return Ok(None) };
    let pdu = wait!(read_pdu(ctx)?);
    let _ = wait!(read_crc(ctx)?);
    Ok(Some(RequestFrame::from_parts(0, slave, pdu)))
}

fn write_rtu_frame(ctx: &mut WriteCtx, frame: &ResponseFrame) -> Result<(), Error> {
    write_u8(ctx, frame.slave).unwrap();
    write_pdu(ctx, &frame.pdu).unwrap();
    write_crc(ctx).unwrap();
    Ok(())
}

fn read_net_frame(ctx: &mut ReadCtx) -> Result<Option<RequestFrame>, Error> {
    let header = wait!(read_mbap(ctx)?);
    let pdu = wait!(read_pdu(ctx)?);
    Ok(Some(RequestFrame {
        id: header.id,
        slave: header.slave,
        pdu,
    }))
}

fn write_net_frame(ctx: &mut WriteCtx, frame: &ResponseFrame) -> Result<(), Error> {
    write_mbap(ctx, frame).unwrap();
    write_u8(ctx, frame.slave).unwrap();
    write_pdu(ctx, &frame.pdu).unwrap();
    Ok(())
}

fn frame_ok<T, E>(frame: &Result<Option<T>, E>) -> bool {
    matches!(frame, Ok(Some(_)))
}

fn frame_err<T, E>(frame: &Result<Option<T>, E>) -> bool {
    matches!(frame, Err(_))
}

fn frame_in_prog<T, E>(frame: &Result<Option<T>, E>) -> bool {
    matches!(frame, Ok(None))
}

#[derive(Debug, PartialEq)]
pub enum CodecMode {
    Rtu,
    Net,
}

#[derive(Debug, PartialEq)]
pub enum CodecFlowType {
    Packet,
    Stream,
}

impl CodecFlowType {
    fn is_packet(&self) -> bool {
        matches!(self, CodecFlowType::Packet)
    }
}

pub struct SlaveCodec {
    mode: CodecMode,
    data: CodecFlowType,
}

impl SlaveCodec {
    pub fn new_rtu() -> SlaveCodec {
        SlaveCodec {
            mode: CodecMode::Rtu,
            data: CodecFlowType::Stream,
        }
    }

    pub fn new_tcp() -> SlaveCodec {
        SlaveCodec {
            mode: CodecMode::Net,
            data: CodecFlowType::Stream,
        }
    }

    pub fn new_udp() -> SlaveCodec {
        SlaveCodec {
            mode: CodecMode::Net,
            data: CodecFlowType::Packet,
        }
    }
    fn advance_buffer(
        &self,
        src: &mut BytesMut,
        msg: &Result<Option<RequestFrame>, Error>,
        processed: usize,
    ) {
        if frame_ok(msg) {
            src.advance(processed);
        } else {
            let reset = frame_err(msg) || (frame_in_prog(msg) && self.data.is_packet());
            if reset {
                src.clear();
            }
        }
    }
}

impl Decoder for SlaveCodec {
    type Item = RequestFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut ctx = ReadCtx::new(src);
        let res = match self.mode {
            CodecMode::Rtu => read_rtu_frame(&mut ctx),
            CodecMode::Net => read_net_frame(&mut ctx),
        };

        self.advance_buffer(src, &res, ctx.processed());
        res
    }
}

impl Encoder<ResponseFrame> for SlaveCodec {
    type Error = Error;
    fn encode(&mut self, frame: ResponseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let res = match self.mode {
            CodecMode::Rtu => {
                resize_buffer(dst, frame.pdu.len() + 3);
                write_rtu_frame(&mut WriteCtx::new(dst.as_mut()), &frame)
            }
            CodecMode::Net => {
                resize_buffer(dst, frame.pdu.len() + 7);
                write_net_frame(&mut WriteCtx::new(dst.as_mut()), &frame)
            }
        };
        res
    }
}

#[cfg(test)]
mod test {
    use super::SlaveCodec;
    use super::{
        read_mbap, read_net_frame, read_rtu_frame, write_crc, Error, ReadCtx, ResponseFrame,
        WriteCtx,
    };
    use crate::data::coils::CoilsSlice;
    use crate::frame::prelude::*;
    use bytes::{Buf, BytesMut};
    use tokio_util::codec::{Decoder, Encoder};

    #[test]
    fn read_rtu_frame_empty() {
        let buffer = [];
        let frame = read_rtu_frame(&mut ReadCtx::new(&buffer));
        assert!(frame.is_ok());
        assert!(frame.unwrap().is_none());
    }

    #[test]
    fn read_rtu_frame_short1() {
        let buffer = [0x1];
        let frame = read_rtu_frame(&mut ReadCtx::new(&buffer));
        assert!(frame.is_ok());
        assert!(frame.unwrap().is_none());
    }

    #[test]
    fn read_rtu_frame_short2() {
        let buffer = [0x1, 0x1];
        let frame = read_rtu_frame(&mut ReadCtx::new(&buffer));
        assert!(frame.is_ok());
        assert!(frame.unwrap().is_none());
    }

    #[test]
    fn read_rtu_frame_fc1() {
        let buffer = [0x11, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x84];
        let frame = read_rtu_frame(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        assert_eq!(frame.id, 0);
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            RequestPdu::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x13);
                assert_eq!(nobjs, 37);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_rtu_frame_wrong_crc() {
        let check = [
            vec![0x11, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x85],
            vec![0x11, 0x02, 0x00, 0xC4, 0x00, 0x16, 0xBA, 0xAA],
            vec![0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x88],
        ];

        for rec in check {
            let frame = read_rtu_frame(&mut ReadCtx::new(&rec));
            match frame {
                Err(Error::InvalidCrc) => {}
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn read_rtu_frame_part_crc() {
        let check = [
            vec![0x11, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E],
            vec![0x11, 0x02, 0x00, 0xC4, 0x00, 0x16, 0xBA],
            vec![0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76],
        ];

        for rec in check {
            let res = read_rtu_frame(&mut ReadCtx::new(&rec));
            match res {
                Ok(None) => {}
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn read_net_frame_fc1() {
        let buffer = [
            0x0, 0x1, 0x0, 0x0, 0x0, 0x6, 0x11, 0x01, 0x00, 0x13, 0x00, 0x25,
        ];
        let frame = read_net_frame(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        assert_eq!(frame.id, 1);
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            RequestPdu::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x13);
                assert_eq!(nobjs, 37);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn decode_fc1() {
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x84];
        let mut buffer = BytesMut::from(&input[..]);
        let frame = SlaveCodec::new_rtu().decode(&mut buffer).unwrap().unwrap();
        match frame.pdu {
            RequestPdu::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x13);
                assert_eq!(nobjs, 0x25);
            }
            _ => unreachable!(),
        }
        assert_eq!(buffer.len(), 0);
    }
    #[test]
    fn decode_fc1_crc_err() {
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x1E, 0x84];
        let mut buffer = BytesMut::from(&input[..]);
        let frame = SlaveCodec::new_rtu().decode(&mut buffer);
        match frame {
            Err(_) => {}
            _ => unreachable!(),
        }
        assert_eq!(buffer.len(), 0);
    }
    #[test]
    fn decode_fc1_crc_not_full() {
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E];
        let mut buffer = BytesMut::from(&input[..]);
        let frame = SlaveCodec::new_rtu().decode(&mut buffer);
        match frame {
            Ok(None) => (),
            _ => unreachable!(),
        }
        assert_eq!(buffer.len(), 7);
    }

    #[test]
    fn write_data_crc() {
        let control = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0E, 0x84];
        let input = [0x11u8, 0x01, 0x00, 0x13, 0x00, 0x25];
        let mut output = [0u8; 255];
        let mut ctx = WriteCtx::new(&mut output);
        for b in input {
            ctx.write_u8(b).unwrap();
        }
        let _ = write_crc(&mut ctx).unwrap();
        let pos = ctx.processed();
        assert_eq!(output[..pos], control)
    }

    #[test]
    fn encode_rtu_fc1() {
        let control = [0x11u8, 0x01, 0x05, 0xCD, 0x6B, 0xB2, 0x0E, 0x1B, 0x45, 0xE6];
        let mut buffer = BytesMut::with_capacity(512);
        let frame = ResponseFrame::new(
            0x11,
            ResponsePdu::read_coils(CoilsSlice::new(&[0xCDu8, 0x6B, 0xB2, 0x0E, 0x1B], 37)),
        );
        SlaveCodec::new_rtu().encode(frame, &mut buffer).unwrap();
        assert_eq!(10, buffer.chunk().len());
        assert_eq!(control, buffer.chunk());
    }

    #[test]
    fn encode_net_fc1() {
        let control = [
            0x0, 0x1, 0x0, 0x0, 0x0, 0x8, 0x11u8, 0x01, 0x05, 0xCD, 0x6B, 0xB2, 0x0E, 0x1B,
        ];
        let mut buffer = BytesMut::with_capacity(512);
        let frame = ResponseFrame::from_parts(
            0x1,
            0x11,
            ResponsePdu::read_coils(CoilsSlice::new(&[0xCDu8, 0x6B, 0xB2, 0x0E, 0x1B], 37)),
        );
        SlaveCodec::new_tcp().encode(frame, &mut buffer).unwrap();
        assert_eq!(14, buffer.chunk().len());
        assert_eq!(control, buffer.chunk());
    }

    #[test]
    fn mbap_part() {
        let buffer = [0x0, 0x1, 0x0, 0x0];
        let res = read_mbap(&mut ReadCtx::new(&buffer));
        assert!(res.is_ok());
    }

    #[test]
    fn mbap_part_err() {
        let buffer = [0x0, 0x1, 0x0, 0x3];
        let res = read_mbap(&mut ReadCtx::new(&buffer));
        assert!(res.is_ok());
    }
}
