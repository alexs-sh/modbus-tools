use super::{common, data::Data, exception::Code};

#[derive(Debug, PartialEq)]
pub enum ResponsePDU {
    /// 0x1
    ReadCoils { nobjs: u16, data: Data },

    /// 0x2
    ReadDiscreteInputs { nobjs: u16, data: Data },

    /// 0x3
    ReadHoldingRegisters { nobjs: u16, data: Data },

    /// 0x4
    ReadInputRegisters { nobjs: u16, data: Data },

    /// 0x5
    WriteSingleCoil { address: u16, value: bool },

    /// 0x6
    WriteSingleRegister { address: u16, value: u16 },

    /// 0xF
    WriteMultipleCoils { address: u16, nobjs: u16 },

    /// 0x10
    WriteMultipleRegisters { address: u16, nobjs: u16 },

    /// Another functino code or not parsed PDU
    Raw { function: u8, data: Data },

    /// Exception,
    Exception { function: u8, code: Code },
}

#[derive(Debug, PartialEq)]
pub struct ResponseFrame {
    pub id: Option<u16>,
    /// message id (only ModbusTCP)
    pub slave: u8,
    pub pdu: ResponsePDU,
}

impl ResponseFrame {
    pub fn rtu(slave: u8, pdu: ResponsePDU) -> ResponseFrame {
        ResponseFrame {
            id: None,
            slave,
            pdu,
        }
    }

    pub fn net(id: u16, slave: u8, pdu: ResponsePDU) -> ResponseFrame {
        ResponseFrame {
            id: Some(id),
            slave,
            pdu,
        }
    }
}

impl ResponsePDU {
    pub fn len(&self) -> usize {
        match self {
            ResponsePDU::ReadCoils { data, .. }
            | ResponsePDU::ReadDiscreteInputs { data, .. }
            | ResponsePDU::ReadHoldingRegisters { data, .. }
            | ResponsePDU::ReadInputRegisters { data, .. } => 2 + data.len(),
            ResponsePDU::WriteSingleCoil { .. }
            | ResponsePDU::WriteSingleRegister { .. }
            | ResponsePDU::WriteMultipleCoils { .. }
            | ResponsePDU::WriteMultipleRegisters { .. } => 5,
            ResponsePDU::Raw { data, .. } => 1 + data.len(),
            ResponsePDU::Exception { .. } => 3,
        }
    }
}

impl ResponsePDU {
    /// 0x1
    pub fn read_coils(coils: &[bool]) -> ResponsePDU {
        ResponsePDU::read_coils_inner(1, coils)
    }

    /// 0x2
    pub fn read_discrete_inputs(coils: &[bool]) -> ResponsePDU {
        ResponsePDU::read_coils_inner(2, coils)
    }

    /// 0x3
    pub fn read_holding_registers(registers: &[u16]) -> ResponsePDU {
        ResponsePDU::read_registers_inner(3, registers)
    }

    /// 0x4
    pub fn read_input_registers(registers: &[u16]) -> ResponsePDU {
        ResponsePDU::read_registers_inner(4, registers)
    }

    /// 0x5
    pub fn write_single_coil(address: u16, value: bool) -> ResponsePDU {
        ResponsePDU::WriteSingleCoil { address, value }
    }

    /// 0x6
    pub fn write_single_register(address: u16, value: u16) -> ResponsePDU {
        ResponsePDU::WriteSingleRegister { address, value }
    }

    /// 0xF
    pub fn write_multiple_coils(address: u16, nobjs: u16) -> ResponsePDU {
        assert!(common::ncoils_check(nobjs));
        ResponsePDU::WriteMultipleCoils { address, nobjs }
    }

    /// 0x10
    pub fn write_multiple_registers(address: u16, nobjs: u16) -> ResponsePDU {
        assert!(common::nregs_check(nobjs));
        ResponsePDU::WriteMultipleRegisters { address, nobjs }
    }

    /// make response with exception
    pub fn exception(func: u8, code: Code) -> ResponsePDU {
        ResponsePDU::Exception {
            function: func | 0x80,
            code,
        }
    }

    /// raw
    pub fn raw(func: u8, data: Data) -> ResponsePDU {
        ResponsePDU::Raw {
            function: func,
            data,
        }
    }

    fn read_coils_inner(func: u8, coils: &[bool]) -> ResponsePDU {
        let nobjs = coils.len() as u16;

        assert!(common::ncoils_check(nobjs));
        assert!(func == 0x1 || func == 0x2);

        match func {
            0x1 => ResponsePDU::ReadCoils {
                nobjs,
                data: Data::coils(coils),
            },
            0x2 => ResponsePDU::ReadDiscreteInputs {
                nobjs,
                data: Data::coils(coils),
            },
            _ => unreachable!(),
        }
    }

    fn read_registers_inner(func: u8, registers: &[u16]) -> ResponsePDU {
        let nobjs = registers.len() as u16;
        assert!(common::nregs_check(nobjs));
        assert!(func == 0x3 || func == 0x4);

        match func {
            0x3 => ResponsePDU::ReadHoldingRegisters {
                nobjs,
                data: Data::registers(registers),
            },
            0x4 => ResponsePDU::ReadInputRegisters {
                nobjs,
                data: Data::registers(registers),
            },
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common;

    #[test]
    fn build_fc1_response_builder() {
        let nbits = 37;
        let bytes = [0xCD, 0x6B, 0xB2, 0x0E, 0x1B];
        let bits = common::bits_from_bytes(&bytes, nbits);
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::read_coils(&bits));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::ReadCoils { nobjs, data } => {
                assert_eq!(nobjs, nbits as u16);
                assert_eq!(data.len(), 0x5);
                assert_eq!(data.get_u8(0).unwrap(), 0xCD);
                assert_eq!(data.get_u8(1).unwrap(), 0x6B);
                assert_eq!(data.get_u8(2).unwrap(), 0xB2);
                assert_eq!(data.get_u8(3).unwrap(), 0x0E);
                assert_eq!(data.get_u8(4).unwrap(), 0x1B);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc1_response() {
        let nbits = 37;
        let bytes = [0xCD, 0x6B, 0xB2, 0x0E, 0x1B];
        let bits = common::bits_from_bytes(&bytes, nbits);
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::read_coils(&bits));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::ReadCoils { nobjs, data } => {
                assert_eq!(nobjs, nbits as u16);
                assert_eq!(data.len(), 0x5);
                assert_eq!(data.get_u8(0).unwrap(), 0xCD);
                assert_eq!(data.get_u8(1).unwrap(), 0x6B);
                assert_eq!(data.get_u8(2).unwrap(), 0xB2);
                assert_eq!(data.get_u8(3).unwrap(), 0x0E);
                assert_eq!(data.get_u8(4).unwrap(), 0x1B);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc2_response() {
        let nbits = 37;
        let bytes = [0xCD, 0x6B, 0xB2, 0x0E, 0x1B];
        let bits = common::bits_from_bytes(&bytes, nbits);
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::read_discrete_inputs(&bits));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::ReadDiscreteInputs { nobjs, data } => {
                assert_eq!(nobjs, nbits as u16);
                assert_eq!(data.len(), 0x5);
                assert_eq!(data.get_u8(0).unwrap(), 0xCD);
                assert_eq!(data.get_u8(1).unwrap(), 0x6B);
                assert_eq!(data.get_u8(2).unwrap(), 0xB2);
                assert_eq!(data.get_u8(3).unwrap(), 0x0E);
                assert_eq!(data.get_u8(4).unwrap(), 0x1B);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc3_response() {
        let registers = [1, 2, 0xFFFF];
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::read_holding_registers(&registers));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::ReadHoldingRegisters { nobjs, data } => {
                assert_eq!(nobjs, 3);
                assert_eq!(data.len(), 0x6);
                assert_eq!(data.get_u16(0).unwrap(), 1);
                assert_eq!(data.get_u16(1).unwrap(), 2);
                assert_eq!(data.get_u16(2).unwrap(), 0xFFFF);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc4_response() {
        let registers = [1, 2, 3, 0xFFFF];
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::read_input_registers(&registers));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::ReadInputRegisters { nobjs, data } => {
                assert_eq!(nobjs, 4);
                assert_eq!(data.len(), 0x8);
                assert_eq!(data.get_u16(0).unwrap(), 1);
                assert_eq!(data.get_u16(1).unwrap(), 2);
                assert_eq!(data.get_u16(2).unwrap(), 3);
                assert_eq!(data.get_u16(3).unwrap(), 0xFFFF);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc5_response() {
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::write_single_coil(0x00AC, true));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(value, true);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc6_response() {
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::write_single_register(0x00AC, 0x123));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::WriteSingleRegister { address, value } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(value, 0x123);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc15_response() {
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::write_multiple_coils(0x00AC, 0x10));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::WriteMultipleCoils { address, nobjs } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(nobjs, 0x10);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc16_response() {
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::write_multiple_registers(0x00AC, 0x11));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::WriteMultipleRegisters { address, nobjs } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(nobjs, 0x11);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_exception_response() {
        let frame = ResponseFrame::rtu(0x11, ResponsePDU::exception(0x3, Code::IllegalFunction));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePDU::Exception { function, code } => {
                assert_eq!(function, 0x83);
                assert_eq!(code, Code::IllegalFunction);
            }
            _ => unreachable!(),
        }
    }
}
