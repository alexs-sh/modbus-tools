extern crate frame;
use crate::error::Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::{Buf, BytesMut};
use frame::common;
use frame::{
    data::BytesCursor, data::CoilsCursor, data::Data, data::RegistersCursorBe, RequestPdu,
    ResponseFrame, ResponsePdu, COIL_OFF, COIL_ON, MAX_DATA_SIZE, MAX_NCOILS, MAX_NREGS,
};
use std::io::Cursor;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Default)]
pub struct PduRequestCodec;

impl Decoder for PduRequestCodec {
    type Item = RequestPdu;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let src = &mut Cursor::new(src.as_ref());
        src.read_u8().map_or(Ok(None), |func| match func {
            0x1 => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                check_ncoils(v2)?;
                Ok(Some(RequestPdu::read_coils(v1, v2)))
            }),
            0x2 => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                check_ncoils(v2)?;
                Ok(Some(RequestPdu::read_discrete_inputs(v1, v2)))
            }),
            0x3 => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                check_nregs(v2)?;
                Ok(Some(RequestPdu::read_holding_registers(v1, v2)))
            }),
            0x4 => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                check_nregs(v2)?;
                Ok(Some(RequestPdu::read_input_registers(v1, v2)))
            }),
            0x5 => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                let cmd = coil_cmd(v2)?;
                Ok(Some(RequestPdu::write_single_coil(v1, cmd)))
            }),
            0x6 => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                Ok(Some(RequestPdu::write_single_register(v1, v2)))
            }),
            0xF => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                src.read_u8().map_or(Ok(None), |nbytes| {
                    let address = v1;
                    let nobjs = v2;

                    check_ncoils(nobjs)?;
                    check_nbytes(common::ncoils_len(nobjs), nbytes as usize)?;

                    let nbytes = nbytes as usize;
                    if src.remaining() >= nbytes {
                        Ok(Some(RequestPdu::write_multiple_coils(
                            address,
                            CoilsCursor::new(src, nobjs),
                        )))
                    } else {
                        Ok(None)
                    }
                })
            }),

            0x10 => prefix_from_cursor(src).map_or(Ok(None), |(v1, v2)| {
                src.read_u8().map_or(Ok(None), |nbytes| {
                    let address = v1;
                    let nobjs = v2;

                    check_nregs(nobjs)?;
                    check_nbytes(common::nregs_len(nobjs), nbytes as usize)?;

                    let nbytes = nbytes as usize;
                    if src.remaining() >= nbytes {
                        Ok(Some(RequestPdu::write_multiple_registers(
                            address,
                            RegistersCursorBe::new(src, nobjs),
                        )))
                    } else {
                        Ok(None)
                    }
                })
            }),

            0x2b => src.read_u8().map_or(Ok(None), |mei_type| match mei_type {
                0xE => Ok(Some(RequestPdu::encapsulated_interface_transport(
                    mei_type,
                    BytesCursor::new(src, 1),
                ))),
                0xD => Ok(Some(RequestPdu::encapsulated_interface_transport(
                    mei_type,
                    BytesCursor::new(src, src.remaining() as u16),
                ))),
                _ => Err(Error::InvalidData),
            }),

            func => {
                let min = std::cmp::min(src.remaining(), MAX_DATA_SIZE);
                let mut data = Data::raw_empty(min);
                src.copy_to_slice(data.get_mut());
                Ok(Some(RequestPdu::raw(func, data)))
            }
        })
    }
}

impl Encoder<ResponseFrame> for PduRequestCodec {
    type Error = Error;
    fn encode(&mut self, _msg: ResponseFrame, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

#[derive(Default)]
pub struct PduResponseCodec;

impl Decoder for PduResponseCodec {
    type Item = ResponsePdu;
    type Error = Error;

    fn decode(&mut self, _src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        unimplemented!()
    }
}

impl Encoder<ResponsePdu> for PduResponseCodec {
    type Error = Error;
    fn encode(&mut self, src: ResponsePdu, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let dst = &mut Cursor::new(dst.as_mut());
        match src {
            ResponsePdu::ReadCoils { data, .. } => {
                check_capacity(data.len() + 2, dst)?;
                dst.write_u8(0x1)?;
                dst.write_u8(data.len() as u8)?;
                write_coils_data(&data, dst);
                Ok(())
            }
            ResponsePdu::ReadDiscreteInputs { data, .. } => {
                check_capacity(data.len() + 2, dst)?;
                dst.write_u8(0x2)?;
                dst.write_u8(data.len() as u8)?;
                write_coils_data(&data, dst);
                Ok(())
            }
            ResponsePdu::ReadHoldingRegisters { data, .. } => {
                check_capacity(data.len() + 2, dst)?;
                dst.write_u8(0x3)?;
                dst.write_u8(data.len() as u8)?;
                write_regs_data(&data, dst);
                Ok(())
            }
            ResponsePdu::ReadInputRegisters { data, .. } => {
                check_capacity(data.len() + 2, dst)?;
                dst.write_u8(0x4)?;
                dst.write_u8(data.len() as u8)?;
                write_regs_data(&data, dst);
                Ok(())
            }
            ResponsePdu::WriteSingleCoil { address, value } => {
                check_capacity(5, dst)?;
                dst.write_u8(0x5)?;
                dst.write_u16::<BigEndian>(address)?;
                dst.write_u16::<BigEndian>(if value { COIL_ON } else { COIL_OFF })?;
                Ok(())
            }
            ResponsePdu::WriteSingleRegister { address, value } => {
                check_capacity(5, dst)?;
                dst.write_u8(0x6)?;
                dst.write_u16::<BigEndian>(address)?;
                dst.write_u16::<BigEndian>(value)?;
                Ok(())
            }

            ResponsePdu::WriteMultipleCoils { address, nobjs } => {
                check_capacity(5, dst)?;
                dst.write_u8(0xF)?;
                dst.write_u16::<BigEndian>(address)?;
                dst.write_u16::<BigEndian>(nobjs)?;
                Ok(())
            }
            ResponsePdu::WriteMultipleRegisters { address, nobjs } => {
                check_capacity(5, dst)?;
                dst.write_u8(0x10)?;
                dst.write_u16::<BigEndian>(address)?;
                dst.write_u16::<BigEndian>(nobjs)?;
                Ok(())
            }
            ResponsePdu::Exception { function, code } => {
                check_capacity(2, dst)?;
                dst.write_u8(function)?;
                dst.write_u8(code as u8)?;
                Ok(())
            }
            ResponsePdu::EncapsulatedInterfaceTransport { mei_type, data } => {
                check_capacity(2 + data.len(), dst)?;
                dst.write_u8(0x2b)?;
                dst.write_u8(mei_type)?;
                write_bytes_data(&data, dst);
                Ok(())
            }
            _ => unreachable!(),
        }
    }
}

fn prefix_from_cursor(src: &mut Cursor<&[u8]>) -> Option<(u16, u16)> {
    if src.remaining() >= 4 {
        let v1 = src.read_u16::<BigEndian>().unwrap();
        let v2 = src.read_u16::<BigEndian>().unwrap();
        Some((v1, v2))
    } else {
        None
    }
}

fn check_ncoils(nobjs: u16) -> Result<(), Error> {
    if nobjs > 0 && nobjs as usize <= MAX_NCOILS {
        Ok(())
    } else {
        Err(Error::InvalidData)
    }
}

fn check_nregs(nobjs: u16) -> Result<(), Error> {
    if nobjs > 0 && nobjs as usize <= MAX_NREGS {
        Ok(())
    } else {
        Err(Error::InvalidData)
    }
}

fn check_nbytes(requested: usize, actual: usize) -> Result<(), Error> {
    if requested == actual {
        Ok(())
    } else {
        Err(Error::InvalidData)
    }
}

fn coil_cmd(value: u16) -> Result<bool, Error> {
    let valid = [COIL_ON, COIL_OFF].iter().any(|x| x == &value);
    if valid {
        Ok(value == COIL_ON)
    } else {
        Err(Error::InvalidData)
    }
}

fn check_capacity(requested: usize, dst: &mut Cursor<&mut [u8]>) -> Result<(), Error> {
    if requested > dst.remaining() {
        Err(Error::BufferToSmall)
    } else {
        Ok(())
    }
}

fn write_coils_data(data: &Data, dst: &mut Cursor<&mut [u8]>) {
    for i in 0..data.len() {
        dst.write_u8(data.get_u8(i).unwrap()).unwrap();
    }
}

fn write_regs_data(data: &Data, dst: &mut Cursor<&mut [u8]>) {
    let regs = data.len() / 2;
    for i in 0..regs {
        dst.write_u16::<BigEndian>(data.get_u16(i).unwrap())
            .unwrap();
    }
}

fn write_bytes_data(data: &Data, dst: &mut Cursor<&mut [u8]>) {
    let bytes = data.len();
    for i in 0..bytes {
        dst.write_u8(data.get_u8(i).unwrap()).unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use frame::common;
    use frame::exception::Code;

    #[test]
    fn pack_fc1() {
        let payload = [0xCDu8, 0x6B, 0xB2, 0x0E, 0x1B];
        let control = [0x01u8, 0x05, 0xCD, 0x6B, 0xB2, 0x0E, 0x1B];
        let bits = common::bits_from_bytes(&payload, 37);
        let pdu = ResponsePdu::read_coils(bits.as_slice());
        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn pack_fc3() {
        let regs = [0xAE41u16, 0x5652, 0x4340];
        let control = [0x03u8, 0x06, 0xAE, 0x41, 0x56, 0x52, 0x43, 0x40];
        let pdu = ResponsePdu::read_holding_registers(&regs[..]);
        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn pack_fc4() {
        let regs = [0x000Au16];
        let control = [0x04, 0x02, 0x00, 0x0A];
        let pdu = ResponsePdu::read_input_registers(&regs[..]);

        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn pack_fc5() {
        let control = [0x05, 0x00, 0xAC, 0xFF, 0x00];
        let pdu = ResponsePdu::write_single_coil(0x00AC, true);

        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn pack_fc6() {
        let control = [0x06, 0x00, 0x01, 0x00, 0x03];
        let pdu = ResponsePdu::write_single_register(0x0001, 0x0003);
        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn pack_fc15() {
        let control = [0x0F, 0x00, 0x13, 0x00, 0x0A];
        let pdu = ResponsePdu::write_multiple_coils(0x0013, 0x000A);
        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn pack_fc16() {
        let control = [0x10, 0x00, 0x01, 0x00, 0x02];
        let pdu = ResponsePdu::write_multiple_registers(0x0001, 0x0002);

        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn pack_exception() {
        let control = [0x81, 0x02];
        let pdu = ResponsePdu::exception(0x1, Code::IllegalDataAddress);

        let mut buffer = BytesMut::new();
        buffer.resize(control.len(), 0);
        PduResponseCodec::default()
            .encode(pdu, &mut buffer)
            .unwrap();
        assert_eq!(&control[..], buffer.as_ref());
    }

    #[test]
    fn parse_fc_unk() {
        let input = [0xF0u8, 0x00, 0x01, 0x0];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);
        assert!(pdu.is_ok());
        match pdu {
            Ok(Some(RequestPdu::Raw { function, data })) => {
                assert_eq!(function, 0xF0);
                assert_eq!(data.len(), 3);
            }
            _ => {
                unreachable!()
            }
        }
    }

    #[test]
    fn parse_fc1_req() {
        let input = [0x1, 0x00, 0x01, 0x0, 0x10];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();
        let _ = match pdu {
            RequestPdu::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x0001);
                assert_eq!(nobjs, 0x10);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc1_req_short() {
        let input = [0x1, 0x00, 0x01, 0x0];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap();
        assert_eq!(pdu, None);
    }

    #[test]
    fn parse_fc2_req() {
        let input = [0x2, 0x01, 0x02, 0x0, 0x11];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();
        let _ = match pdu {
            RequestPdu::ReadDiscreteInputs { address, nobjs } => {
                assert_eq!(address, 0x0102);
                assert_eq!(nobjs, 0x11);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc3_req() {
        let input = [0x3, 0x00, 0x03, 0x0, 0x12];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();
        let _ = match pdu {
            RequestPdu::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x03);
                assert_eq!(nobjs, 0x12);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc4_req() {
        let input = [0x4, 0x00, 0x04, 0x0, 0x13];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();
        let _ = match pdu {
            RequestPdu::ReadInputRegisters { address, nobjs } => {
                assert_eq!(address, 0x04);
                assert_eq!(nobjs, 0x13);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc5_req_on() {
        let input = [0x5, 0x00, 0x05, 0xFF, 0x00];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();

        let _ = match pdu {
            RequestPdu::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0x05);
                assert_eq!(value, true);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc5_req_off() {
        let input = [0x5, 0x00, 0x05, 0x00, 0x00];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();
        let _ = match pdu {
            RequestPdu::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0x05);
                assert_eq!(value, false);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc5_req_inv() {
        let input = [0x5, 0x00, 0x05, 0x00, 0x01];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);
        assert!(pdu.is_err());
    }

    #[test]
    fn parse_fc6_req() {
        let input = [0x6, 0x00, 0x06, 0xFF, 0x00];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();
        let _ = match pdu {
            RequestPdu::WriteSingleRegister { address, value } => {
                assert_eq!(address, 0x6);
                assert_eq!(value, 0xFF00);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc15_req() {
        let input = [0xF, 0x00, 0x0F, 0x00, 0xA, 0x2, 0xCD, 0x01];
        let values = [
            true, false, true, true, false, false, true, true, true, false,
        ];

        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();

        let _ = match pdu {
            RequestPdu::WriteMultipleCoils {
                address,
                nobjs,
                data,
            } => {
                assert_eq!(address, 0xF);
                assert_eq!(nobjs, 0xA);

                for (n, b) in values.iter().enumerate() {
                    assert_eq!(data.get_bit(n).unwrap(), *b);
                }
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc15_inv1() {
        // invalid number of objects
        let input = [0xF, 0x00, 0x0F, 0x00, 0x20, 0x2, 0xCD, 0x01];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), Error::InvalidData);
    }

    #[test]
    fn parse_fc15_inv2() {
        // invalid number of bytes
        let input = [0xF, 0x00, 0x0F, 0x00, 0xA, 0x1, 0xCD, 0x01];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), Error::InvalidData);
    }

    #[test]
    fn parse_fc15_part() {
        // invalid number of bytes
        let input = [0xF, 0x00, 0x0F, 0x00, 0x1D, 0x4, 0xCD, 0x01];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);

        assert!(pdu.is_ok());
        assert_eq!(pdu.unwrap(), None);
    }

    #[test]
    fn parse_fc16_req() {
        let input = [0x10, 0x00, 0x10, 0x00, 0x2, 0x4, 0x00, 0xFF, 0xFF, 0x00];
        let values = [0x00FF, 0xFF00];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes).unwrap().unwrap();

        let _ = match pdu {
            RequestPdu::WriteMultipleRegisters {
                address,
                nobjs,
                data,
            } => {
                assert_eq!(address, 0x10);
                assert_eq!(nobjs, 0x2);

                for (n, r) in values.iter().enumerate() {
                    assert_eq!(data.get_u16(n).unwrap(), *r);
                }
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn parse_fc16_req_inv1() {
        // invalid number of bytes of payload
        let input = [0x10, 0x00, 0x10, 0x00, 0x2, 0x3, 0x00, 0xFF, 0xFF, 0x00];

        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), Error::InvalidData);
    }

    #[test]
    fn parse_fc16_req_inv2() {
        // invalid number of register
        let input = [0x10, 0x00, 0x10, 0x00, 0x1, 0x4, 0x00, 0xFF, 0xFF, 0x00];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), Error::InvalidData);
    }

    #[test]
    fn parse_fc16_req_part() {
        // partial message
        let input = [0x10, 0x00, 0x10, 0x00, 0x3, 0x6, 0x00, 0xFF, 0xFF, 0x00];
        let bytes = &mut BytesMut::from(&input[..]);
        let pdu = PduRequestCodec::default().decode(bytes);

        assert!(pdu.is_ok());
        assert_eq!(pdu.unwrap(), None);
    }
}
