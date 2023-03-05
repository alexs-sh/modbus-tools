use crate::codec::context::{ReadCtx, WriteCtx};
use crate::codec::error::Error;
use crate::codec::wait;
use crate::data::{
    bytes::BytesCursor, checks, coils::CoilsCursor, helpers, registers::RegistersCursorBe,
    storage::DataStorage, MAX_DATA_SIZE,
};

use crate::frame::prelude::*;
use bytes::Buf;
use std::io::Cursor;

const COIL_ON: u16 = 0xFF00;
const COIL_OFF: u16 = 0x0000;

pub(crate) fn read_pdu(ctx: &mut ReadCtx) -> Result<Option<RequestPdu>, Error> {
    let func = wait!(ctx.read_u8()); // else { return Ok(None) };
    match func {
        0x1 => {
            let address = wait!(ctx.read_u16_be());
            let nobjs = wait!(ctx.read_u16_be());
            check_coils_count(nobjs)?;
            Ok(Some(RequestPdu::read_coils(address, nobjs)))
        }
        0x2 => {
            let address = wait!(ctx.read_u16_be());
            let nobjs = wait!(ctx.read_u16_be());
            check_coils_count(nobjs)?;
            Ok(Some(RequestPdu::read_discrete_inputs(address, nobjs)))
        }
        0x3 => {
            let address = wait!(ctx.read_u16_be());
            let nobjs = wait!(ctx.read_u16_be());
            check_registers_count(nobjs)?;
            Ok(Some(RequestPdu::read_holding_registers(address, nobjs)))
        }
        0x4 => {
            let address = wait!(ctx.read_u16_be());
            let nobjs = wait!(ctx.read_u16_be());
            check_registers_count(nobjs)?;
            Ok(Some(RequestPdu::read_input_registers(address, nobjs)))
        }
        0x5 => {
            let address = wait!(ctx.read_u16_be());
            let value = wait!(ctx.read_u16_be());
            let value = raw_to_coil(value)?;
            Ok(Some(RequestPdu::write_single_coil(address, value)))
        }
        0x6 => {
            let address = wait!(ctx.read_u16_be());
            let value = wait!(ctx.read_u16_be());
            Ok(Some(RequestPdu::write_single_register(address, value)))
        }
        0xF => {
            let address = wait!(ctx.read_u16_be());
            let nobjs = wait!(ctx.read_u16_be());
            let nbytes = wait!(ctx.read_u8());
            check_coils_count(nobjs)?;
            check_matching(helpers::get_coils_len(nobjs), nbytes as usize)?;
            wait!(ctx.is_enough(nbytes as usize));
            let pdu =
                RequestPdu::write_multiple_coils(address, CoilsCursor::new(&mut ctx.cursor, nobjs));
            Ok(Some(pdu))
        }
        0x10 => {
            let address = wait!(ctx.read_u16_be());
            let nobjs = wait!(ctx.read_u16_be());
            let nbytes = wait!(ctx.read_u8());
            check_registers_count(nobjs)?;
            check_matching(helpers::get_registers_len(nobjs), nbytes as usize)?;
            wait!(ctx.is_enough(nbytes as usize));
            let pdu = RequestPdu::write_multiple_registers(
                address,
                RegistersCursorBe::new(&mut ctx.cursor, nobjs),
            );
            Ok(Some(pdu))
        }
        0x2b => {
            let mei_type = wait!(ctx.read_u8());
            check_mei_type(mei_type)?;
            wait!(ctx.is_enough(1));
            let pdu = match mei_type {
                0xE => RequestPdu::encapsulated_interface_transport(
                    mei_type,
                    BytesCursor::new(&mut ctx.cursor, 1),
                ),
                0xD => {
                    let remain = ctx.remaining() as u16;
                    RequestPdu::encapsulated_interface_transport(
                        mei_type,
                        BytesCursor::new(&mut ctx.cursor, remain),
                    )
                }
                _ => unimplemented!(),
            };
            Ok(Some(pdu))
        }
        _ => {
            let min = std::cmp::min(ctx.remaining(), MAX_DATA_SIZE);
            let mut data = DataStorage::raw_empty(min);
            ctx.cursor.copy_to_slice(data.get_mut());
            Ok(Some(RequestPdu::raw(func, data)))
        }
    }
}

pub(crate) fn write_pdu(ctx: &mut WriteCtx, src: &ResponsePdu) -> Result<Option<()>, Error> {
    match src {
        ResponsePdu::ReadCoils { data, .. } => {
            ctx.is_enough(data.len() + 2).unwrap();
            ctx.write_u8(0x1).unwrap();
            ctx.write_u8(data.len() as u8).unwrap();
            ctx.write_bytes(data.get()).unwrap();
            Ok(Some(()))
        }
        ResponsePdu::ReadDiscreteInputs { data, .. } => {
            ctx.is_enough(data.len() + 2).unwrap();
            ctx.write_u8(0x2).unwrap();
            ctx.write_u8(data.len() as u8).unwrap();
            ctx.write_bytes(data.get()).unwrap();
            Ok(Some(()))
        }
        ResponsePdu::ReadHoldingRegisters { data, .. } => {
            ctx.is_enough(data.len() + 2).unwrap();
            ctx.write_u8(0x3).unwrap();
            ctx.write_u8(data.len() as u8).unwrap();
            ctx.write_data_u16_be(data.get()).unwrap();
            Ok(Some(()))
        }
        ResponsePdu::ReadInputRegisters { data, .. } => {
            ctx.is_enough(data.len() + 2).unwrap();
            ctx.write_u8(0x4).unwrap();
            ctx.write_u8(data.len() as u8).unwrap();
            ctx.write_data_u16_be(data.get()).unwrap();
            Ok(Some(()))
        }
        ResponsePdu::WriteSingleCoil { address, value } => {
            ctx.is_enough(5).unwrap();
            ctx.write_u8(0x5).unwrap();
            ctx.write_u16_be(*address).unwrap();
            ctx.write_u16_be(coil_to_raw(*value)).unwrap();
            Ok(Some(()))
        }
        ResponsePdu::WriteSingleRegister { address, value } => {
            ctx.is_enough(5).unwrap();
            ctx.write_u8(0x6).unwrap();
            ctx.write_u16_be(*address).unwrap();
            ctx.write_u16_be(*value).unwrap();
            Ok(Some(()))
        }

        ResponsePdu::WriteMultipleCoils { address, nobjs } => {
            ctx.is_enough(5).unwrap();
            ctx.write_u8(0xF).unwrap();
            ctx.write_u16_be(*address).unwrap();
            ctx.write_u16_be(*nobjs).unwrap();
            Ok(Some(()))
        }

        ResponsePdu::WriteMultipleRegisters { address, nobjs } => {
            ctx.is_enough(5).unwrap();
            ctx.write_u8(0x10).unwrap();
            ctx.write_u16_be(*address).unwrap();
            ctx.write_u16_be(*nobjs).unwrap();
            Ok(Some(()))
        }

        ResponsePdu::Exception { function, code } => {
            ctx.is_enough(2).unwrap();
            ctx.write_u8(*function | 0x80).unwrap();
            ctx.write_u8(*code as u8).unwrap();
            Ok(Some(()))
        }
        ResponsePdu::EncapsulatedInterfaceTransport { mei_type, data } => {
            ctx.is_enough(2).unwrap();
            ctx.write_u8(0x2b).unwrap();
            ctx.write_u8(*mei_type).unwrap();
            ctx.write_bytes(data.get());
            Ok(Some(()))
        }
        _ => unreachable!(),
    }
}

fn check_coils_count(nobjs: u16) -> Result<(), Error> {
    if checks::check_coils_count(nobjs) {
        Ok(())
    } else {
        Err(Error::InvalidData)
    }
}

fn check_registers_count(nobjs: u16) -> Result<(), Error> {
    if checks::check_registers_count(nobjs) {
        Ok(())
    } else {
        Err(Error::InvalidData)
    }
}

fn check_matching(requested: usize, actual: usize) -> Result<(), Error> {
    if requested == actual {
        Ok(())
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

fn check_mei_type(mei_type: u8) -> Result<(), Error> {
    match mei_type {
        0xE | 0xD => Ok(()),
        _ => Err(Error::InvalidData),
    }
}

fn raw_to_coil(value: u16) -> Result<bool, Error> {
    let valid = [COIL_ON, COIL_OFF].iter().any(|x| x == &value);
    if valid {
        Ok(value == COIL_ON)
    } else {
        Err(Error::InvalidData)
    }
}

fn coil_to_raw(value: bool) -> u16 {
    if value {
        COIL_ON
    } else {
        COIL_OFF
    }
}

#[cfg(test)]
mod test {
    use super::{read_pdu, write_pdu, Error, ReadCtx, RequestPdu, ResponsePdu, WriteCtx};
    use crate::data::prelude::*;
    use crate::frame::exception::Code;
    #[test]
    fn read_pdu_fc1() {
        let buffer = [0x01, 0x00, 0x13, 0x00, 0x25];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x13);
                assert_eq!(nobjs, 37);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_fc2() {
        let buffer = [0x02, 0x00, 0xC4, 0x00, 0x16];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::ReadDiscreteInputs { address, nobjs } => {
                assert_eq!(address, 0xC4);
                assert_eq!(nobjs, 22);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_fc3() {
        let buffer = [0x03, 0x00, 0x6B, 0x00, 0x03];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x6B);
                assert_eq!(nobjs, 3);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_fc4() {
        let buffer = [0x04, 0x00, 0x08, 0x00, 0x01];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::ReadInputRegisters { address, nobjs } => {
                assert_eq!(address, 0x8);
                assert_eq!(nobjs, 1);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_fc5() {
        let buffer = [0x05, 0x00, 0xAC, 0xFF, 0x00];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0xAC);
                assert_eq!(value, true);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_fc6() {
        let buffer = [0x06, 0x00, 0x01, 0x00, 0x03];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::WriteSingleRegister { address, value } => {
                assert_eq!(address, 0x1);
                assert_eq!(value, 3);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_fc15() {
        let buffer = [0x0F, 0x00, 0x13, 0x00, 0x0A, 0x02, 0xCD, 0x01];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::WriteMultipleCoils {
                address,
                nobjs,
                data,
            } => {
                assert_eq!(address, 0x13);
                assert_eq!(nobjs, 10);
                assert_eq!(data.get_u16(0), Some(0x01CD));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_fc16() {
        let buffer = [0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::WriteMultipleRegisters {
                address,
                nobjs,
                data,
            } => {
                assert_eq!(address, 0x1);
                assert_eq!(nobjs, 2);
                assert_eq!(data.get_u16(0), Some(0xA));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn read_pdu_0x2b() {
        let buffer = [0x2B, 0x0E, 0x1];
        let pdu = read_pdu(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        match pdu {
            RequestPdu::EncapsulatedInterfaceTransport { mei_type, data } => {
                assert_eq!(mei_type, 0xE);
                assert_eq!(data.get_u8(0).unwrap(), 0x1);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn read_pdu_parts() {
        let check = [
            vec![0x01],
            vec![0x01, 0x00],
            vec![0x01, 0x00, 0x13],
            vec![0x01, 0x00, 0x13, 0x00],
            vec![0x02],
            vec![0x02, 0x00],
            vec![0x02, 0x00, 0xC4],
            vec![0x02, 0x00, 0xC4, 0x00],
            vec![0x03],
            vec![0x03, 0x00],
            vec![0x03, 0x00, 0xC5],
            vec![0x03, 0x00, 0xC5, 0x00],
            vec![0x04],
            vec![0x04, 0x00],
            vec![0x04, 0x00, 0xC6],
            vec![0x04, 0x00, 0xC6, 0x00],
            vec![0x05],
            vec![0x05, 0x00],
            vec![0x05, 0x00, 0xAC],
            vec![0x05, 0x00, 0xAC, 0xFF],
            vec![0x0F],
            vec![0x0F, 0x00],
            vec![0x0F, 0x00, 0x13],
            vec![0x0F, 0x00, 0x13, 0x00],
            vec![0x0F, 0x00, 0x13, 0x00, 0x0A],
            vec![0x0F, 0x00, 0x13, 0x00, 0x0A, 0x02],
            vec![0x0F, 0x00, 0x13, 0x00, 0x0A, 0x02, 0xCD],
            vec![0x10],
            vec![0x10, 0x00],
            vec![0x10, 0x00, 0x01],
            vec![0x10, 0x00, 0x01, 0x00],
            vec![0x10, 0x00, 0x01, 0x00, 0x02],
            vec![0x10, 0x00, 0x01, 0x00, 0x02, 0x04],
            vec![0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00],
            vec![0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A],
            vec![0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01],
            vec![0x10, 0x00, 0x01, 0x00, 0x04, 0x08, 0x00, 0x0A, 0x01, 0x02],
            vec![0x2B],
            vec![0x2B, 0x0E],
        ];

        for rec in check {
            let mut ctx = ReadCtx::new(rec.as_ref());
            let res = read_pdu(&mut ctx);
            assert!(res.unwrap().is_none());
        }
    }

    #[test]
    fn read_pdu_invalid_data() {
        let check = [
            vec![0x01, 0x00, 0x13, 0xFF, 0x25],
            vec![0x02, 0x00, 0x13, 0xFF, 0x25],
            vec![0x03, 0x00, 0x6B, 0xFF, 0x03],
            vec![0x04, 0x00, 0x6B, 0xFF, 0x03],
            vec![0x04, 0x00, 0x6B, 0xFF, 0x03],
            vec![0x05, 0x00, 0xAC, 0xFF, 0x01],
            vec![0x10, 0x00, 0x01, 0xFF, 0xFF, 0x04, 0x00, 0x0A, 0x01, 0x02],
            vec![0x10, 0x00, 0x01, 0x00, 0x02, 0x03, 0x00, 0x0A, 0x01, 0x02],
            vec![0x0F, 0x00, 0x13, 0x00, 0x0A, 0x01, 0xCD],
            vec![0x0F, 0x00, 0x13, 0xFF, 0x0A, 0x02, 0xCD, 0x01],
            vec![0x2B, 0x01, 0x1],
        ];

        for rec in check {
            let mut ctx = ReadCtx::new(rec.as_ref());
            let res = read_pdu(&mut ctx);
            match res {
                Err(Error::InvalidData) => {}
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn write_pdu_fc1() {
        let control = [0x01u8, 0x05, 0xCD, 0x6B, 0xB2, 0x0E, 0x1B];
        let pdu = ResponsePdu::ReadCoils {
            nobjs: 0x25,
            data: Data::raw(&[0xCDu8, 0x6B, 0xB2, 0x0E, 0x1B]),
        };
        let mut buffer = [0u8; 7];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc2() {
        let control = [0x02, 0x03, 0xAC, 0xDB, 0x35];
        let pdu = ResponsePdu::ReadDiscreteInputs {
            nobjs: 0x16,
            data: Data::raw(&[0xAC, 0xDB, 0x35]),
        };
        let mut buffer = [0u8; 5];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc3() {
        let control = [0x03, 0x06, 0xAE, 0x41, 0x56, 0x52, 0x43, 0x40];
        let pdu = ResponsePdu::ReadHoldingRegisters {
            nobjs: 0x3,
            data: Data::registers([0xAE41u16, 0x5652, 0x4340].as_ref()),
        };
        let mut buffer = [0u8; 8];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc4() {
        let control = [0x04, 0x02, 0x00, 0x0A];
        let pdu = ResponsePdu::ReadInputRegisters {
            nobjs: 0x1,
            data: Data::registers([0xAu16].as_ref()),
        };
        let mut buffer = [0u8; 4];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc5() {
        let control = [0x05, 0x00, 0xAC, 0xFF, 0x00];
        let pdu = ResponsePdu::WriteSingleCoil {
            address: 0xAC,
            value: true,
        };
        let mut buffer = [0u8; 5];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc6() {
        let control = [0x06, 0x00, 0x01, 0x00, 0x03];
        let pdu = ResponsePdu::WriteSingleRegister {
            address: 0x01,
            value: 3,
        };
        let mut buffer = [0u8; 5];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc15() {
        let control = [0x0F, 0x00, 0x13, 0x00, 0x0A];
        let pdu = ResponsePdu::WriteMultipleCoils {
            address: 0x13,
            nobjs: 0xA,
        };
        let mut buffer = [0u8; 5];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc16() {
        let control = [0x10, 0x00, 0x01, 0x00, 0x02];
        let pdu = ResponsePdu::WriteMultipleRegisters {
            address: 0x1,
            nobjs: 0x2,
        };
        let mut buffer = [0u8; 5];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_exception() {
        let control = [0x81, 0x02];
        let pdu = ResponsePdu::Exception {
            function: 0x1,
            code: Code::IllegalDataAddress,
        };
        let mut buffer = [0u8; 2];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }

    #[test]
    fn write_pdu_fc43() {
        let control = [
            0x2b, 0x0E, 0x01, 0x01, 0x0, 0x0, 0x2, 0x1, 0x1, 0x1, 0x2, 0x1, 0x1,
        ];
        let pdu = ResponsePdu::EncapsulatedInterfaceTransport {
            mei_type: 0xE,
            data: Data::raw(&[0x01u8, 0x01, 0x0, 0x0, 0x2, 0x1, 0x1, 0x1, 0x2, 0x1, 0x1]),
        };

        let mut buffer = [0u8; 13];
        write_pdu(&mut WriteCtx::new(&mut buffer), &pdu)
            .unwrap()
            .unwrap();
        assert_eq!(buffer, control);
    }
}
