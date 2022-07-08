extern crate frame;

use crate::common::error::CodecError;
use frame::common;
use frame::{
    coils::CursorCoils, registers::CursorBe, request::RequestPDU, COIL_OFF, COIL_ON, MAX_NCOILS,
    MAX_NREGS,
};

use byteorder::{BigEndian, ReadBytesExt};
use bytes::Buf;
use std::io::Cursor;

pub(crate) fn parse_request(
    func: u8,
    src: &mut Cursor<&[u8]>,
) -> Result<Option<RequestPDU>, CodecError> {
    check_func(func)?;

    let (v1, v2) = if let Some(prefix) = prefix_from_cursor(src) {
        (prefix.0, prefix.1)
    } else {
        return Ok(None);
    };

    match func {
        0x1 => {
            check_ncoils(v2)?;
            Ok(Some(RequestPDU::read_coils(v1, v2)))
        }
        0x2 => {
            check_ncoils(v2)?;
            Ok(Some(RequestPDU::read_discrete_inputs(v1, v2)))
        }
        0x3 => Ok(Some(RequestPDU::read_holding_registers(v1, v2))),
        0x4 => Ok(Some(RequestPDU::read_input_registers(v1, v2))),
        0x5 => {
            let cmd = coil_cmd(v2)?;
            Ok(Some(RequestPDU::write_single_coil(v1, cmd)))
        }
        0x6 => Ok(Some(RequestPDU::write_single_register(v1, v2))),
        0xF => src.read_u8().map_or(Ok(None), |nbytes| {
            let address = v1;
            let nobjs = v2;

            check_ncoils(nobjs)?;
            check_nbytes(common::ncoils_len(nobjs), nbytes as usize)?;

            let nbytes = nbytes as usize;
            if src.remaining() >= nbytes {
                Ok(Some(RequestPDU::write_multiple_coils(
                    address,
                    CursorCoils::new(src, nobjs),
                )))
            } else {
                Ok(None)
            }
        }),

        0x10 => src.read_u8().map_or(Ok(None), |nbytes| {
            let address = v1;
            let nobjs = v2;

            check_nregs(nobjs)?;
            check_nbytes(common::nregs_len(nobjs), nbytes as usize)?;

            let nbytes = nbytes as usize;
            if src.remaining() >= nbytes {
                Ok(Some(RequestPDU::write_multiple_registers(
                    address,
                    CursorBe::new(src, nobjs),
                )))
            } else {
                Ok(None)
            }
        }),
        _ => unreachable!(),
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

fn check_func(func: u8) -> Result<(), CodecError> {
    let res = matches!(func, 0x1 | 0x2 | 0x3 | 0x4 | 0x5 | 0x6 | 0xF | 0x10);
    if res {
        Ok(())
    } else {
        Err(CodecError::UnsupportedFunction)
    }
}

fn check_ncoils(nobjs: u16) -> Result<(), CodecError> {
    if nobjs > 0 && nobjs as usize <= MAX_NCOILS {
        Ok(())
    } else {
        Err(CodecError::InvalidData)
    }
}

fn check_nregs(nobjs: u16) -> Result<(), CodecError> {
    if nobjs > 0 && nobjs as usize <= MAX_NREGS {
        Ok(())
    } else {
        Err(CodecError::InvalidData)
    }
}

fn check_nbytes(requested: usize, actual: usize) -> Result<(), CodecError> {
    if requested == actual {
        Ok(())
    } else {
        Err(CodecError::InvalidData)
    }
}

fn coil_cmd(value: u16) -> Result<bool, CodecError> {
    let valid = [COIL_ON, COIL_OFF].iter().any(|x| x == &value);
    if valid {
        Ok(value == COIL_ON)
    } else {
        Err(CodecError::InvalidData)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_fc_unk() {
        let input = [0xF0, 0x00, 0x01, 0x0];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor);
        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), CodecError::UnsupportedFunction);
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn parse_fc1_req() {
        let input = [0x1, 0x00, 0x01, 0x0, 0x10];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();
        let _ = match pdu {
            RequestPDU::ReadCoils { address, nobjs } => {
                assert_eq!(address, 0x0001);
                assert_eq!(nobjs, 0x10);
            }
            _ => unreachable!(),
        };
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn parse_fc1_req_short() {
        let input = [0x1, 0x00, 0x01, 0x0];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor).unwrap();
        assert_eq!(pdu, None);
    }

    #[test]
    fn parse_fc2_req() {
        let input = [0x2, 0x01, 0x02, 0x0, 0x11];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();
        let _ = match pdu {
            RequestPDU::ReadDiscreteInputs { address, nobjs } => {
                assert_eq!(address, 0x0102);
                assert_eq!(nobjs, 0x11);
            }
            _ => unreachable!(),
        };
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn parse_fc3_req() {
        let input = [0x3, 0x00, 0x03, 0x0, 0x12];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();
        let _ = match pdu {
            RequestPDU::ReadHoldingRegisters { address, nobjs } => {
                assert_eq!(address, 0x03);
                assert_eq!(nobjs, 0x12);
            }
            _ => unreachable!(),
        };
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn parse_fc4_req() {
        let input = [0x4, 0x00, 0x04, 0x0, 0x13];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();
        let _ = match pdu {
            RequestPDU::ReadInputRegisters { address, nobjs } => {
                assert_eq!(address, 0x04);
                assert_eq!(nobjs, 0x13);
            }
            _ => unreachable!(),
        };
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn parse_fc5_req_on() {
        let input = [0x5, 0x00, 0x05, 0xFF, 0x00];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();

        let _ = match pdu {
            RequestPDU::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0x05);
                assert_eq!(value, true);
            }
            _ => unreachable!(),
        };
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn parse_fc5_req_off() {
        let input = [0x5, 0x00, 0x05, 0x00, 0x00];

        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();
        let _ = match pdu {
            RequestPDU::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0x05);
                assert_eq!(value, false);
            }
            _ => unreachable!(),
        };
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn parse_fc5_req_inv() {
        let input = [0x5, 0x00, 0x05, 0x00, 0x01];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor);
        assert!(pdu.is_err());
    }

    #[test]
    fn parse_fc6_req() {
        let input = [0x6, 0x00, 0x06, 0xFF, 0x00];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();
        let _ = match pdu {
            RequestPDU::WriteSingleRegister { address, value } => {
                assert_eq!(address, 0x6);
                assert_eq!(value, 0xFF00);
            }
            _ => unreachable!(),
        };
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn parse_fc15_req() {
        let input = [0xF, 0x00, 0x0F, 0x00, 0xA, 0x2, 0xCD, 0x01];
        let values = [
            true, false, true, true, false, false, true, true, true, false,
        ];

        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();

        let _ = match pdu {
            RequestPDU::WriteMultipleCoils {
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
        assert_eq!(cursor.position(), 8);
    }

    #[test]
    fn parse_fc15_inv1() {
        // invalid number of objects
        let input = [0xF, 0x00, 0x0F, 0x00, 0x20, 0x2, 0xCD, 0x01];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), CodecError::InvalidData);
    }

    #[test]
    fn parse_fc15_inv2() {
        // invalid number of bytes
        let input = [0xF, 0x00, 0x0F, 0x00, 0xA, 0x1, 0xCD, 0x01];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), CodecError::InvalidData);
    }

    #[test]
    fn parse_fc15_part() {
        // invalid number of bytes
        let input = [0xF, 0x00, 0x0F, 0x00, 0x1D, 0x4, 0xCD, 0x01];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();
        let pdu = parse_request(func, &mut cursor);

        assert!(pdu.is_ok());
        assert_eq!(pdu.unwrap(), None);
    }

    #[test]
    fn parse_fc16_req() {
        let input = [0x10, 0x00, 0x10, 0x00, 0x2, 0x4, 0x00, 0xFF, 0xFF, 0x00];
        let values = [0x00FF, 0xFF00];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor).unwrap().unwrap();

        let _ = match pdu {
            RequestPDU::WriteMultipleRegisters {
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
        assert_eq!(cursor.position(), 10);
    }

    #[test]
    fn parse_fc16_req_inv1() {
        // invalid number of bytes of payload
        let input = [0x10, 0x00, 0x10, 0x00, 0x2, 0x3, 0x00, 0xFF, 0xFF, 0x00];

        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), CodecError::InvalidData);
    }

    #[test]
    fn parse_fc16_req_inv2() {
        // invalid number of register
        let input = [0x10, 0x00, 0x10, 0x00, 0x1, 0x4, 0x00, 0xFF, 0xFF, 0x00];

        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor);

        assert!(pdu.is_err());
        assert_eq!(pdu.err().unwrap(), CodecError::InvalidData);
    }

    #[test]
    fn parse_fc16_req_part() {
        // partial message
        let input = [0x10, 0x00, 0x10, 0x00, 0x3, 0x6, 0x00, 0xFF, 0xFF, 0x00];
        let mut cursor = Cursor::new(&input[..]);
        let func = cursor.read_u8().unwrap();

        let pdu = parse_request(func, &mut cursor);

        assert!(pdu.is_ok());
        assert_eq!(pdu.unwrap(), None);
    }
}