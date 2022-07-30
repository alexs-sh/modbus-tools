use super::data::{Coils, Data, Registers};
use super::{common, exception::Code};

#[derive(Debug, PartialEq)]
pub enum ResponsePdu {
    /// 0x1
    ReadCoils {
        nobjs: u16,
        data: Data,
    },

    /// 0x2
    ReadDiscreteInputs {
        nobjs: u16,
        data: Data,
    },

    /// 0x3
    ReadHoldingRegisters {
        nobjs: u16,
        data: Data,
    },

    /// 0x4
    ReadInputRegisters {
        nobjs: u16,
        data: Data,
    },

    /// 0x5
    WriteSingleCoil {
        address: u16,
        value: bool,
    },

    /// 0x6
    WriteSingleRegister {
        address: u16,
        value: u16,
    },

    /// 0xF
    WriteMultipleCoils {
        address: u16,
        nobjs: u16,
    },

    /// 0x10
    WriteMultipleRegisters {
        address: u16,
        nobjs: u16,
    },

    /// 0x2b
    EncapsulatedInterfaceTransport {
        mei_type: u8,
        data: Data,
    },

    Raw {
        function: u8,
        data: Data,
    },

    /// Exception,
    Exception {
        function: u8,
        code: Code,
    },
}

#[derive(Debug, PartialEq)]
pub struct ResponseFrame {
    pub slave: u8,
    pub pdu: ResponsePdu,
}

impl ResponseFrame {
    pub fn new(slave: u8, pdu: ResponsePdu) -> ResponseFrame {
        ResponseFrame { slave, pdu }
    }
}

impl ResponsePdu {
    pub fn len(&self) -> usize {
        match self {
            ResponsePdu::ReadCoils { data, .. }
            | ResponsePdu::ReadDiscreteInputs { data, .. }
            | ResponsePdu::ReadHoldingRegisters { data, .. }
            | ResponsePdu::ReadInputRegisters { data, .. } => 2 + data.len(),
            ResponsePdu::WriteSingleCoil { .. }
            | ResponsePdu::WriteSingleRegister { .. }
            | ResponsePdu::WriteMultipleCoils { .. }
            | ResponsePdu::WriteMultipleRegisters { .. } => 5,
            ResponsePdu::EncapsulatedInterfaceTransport { data, .. } => 2 + data.len(),
            ResponsePdu::Raw { data, .. } => 1 + data.len(),
            ResponsePdu::Exception { .. } => 2,
        }
    }
}

impl ResponsePdu {
    /// 0x1
    pub fn read_coils(coils: impl Coils) -> ResponsePdu {
        ResponsePdu::read_coils_inner(1, coils)
    }

    /// 0x2
    pub fn read_discrete_inputs(coils: impl Coils) -> ResponsePdu {
        ResponsePdu::read_coils_inner(2, coils)
    }

    /// 0x3
    pub fn read_holding_registers(registers: impl Registers) -> ResponsePdu {
        ResponsePdu::read_registers_inner(3, registers)
    }

    /// 0x4
    pub fn read_input_registers(registers: impl Registers) -> ResponsePdu {
        ResponsePdu::read_registers_inner(4, registers)
    }

    /// 0x5
    pub fn write_single_coil(address: u16, value: bool) -> ResponsePdu {
        ResponsePdu::WriteSingleCoil { address, value }
    }

    /// 0x6
    pub fn write_single_register(address: u16, value: u16) -> ResponsePdu {
        ResponsePdu::WriteSingleRegister { address, value }
    }

    /// 0xF
    pub fn write_multiple_coils(address: u16, nobjs: u16) -> ResponsePdu {
        assert!(common::ncoils_check(nobjs));
        ResponsePdu::WriteMultipleCoils { address, nobjs }
    }

    /// 0x10
    pub fn write_multiple_registers(address: u16, nobjs: u16) -> ResponsePdu {
        assert!(common::nregs_check(nobjs));
        ResponsePdu::WriteMultipleRegisters { address, nobjs }
    }

    /// 0x2b
    pub fn encapsulated_interface_transport(mei_type: u8, data: &[u8]) -> ResponsePdu {
        assert!(common::data_bytes_check(data.len()));
        ResponsePdu::EncapsulatedInterfaceTransport {
            mei_type,
            data: Data::raw(data),
        }
    }

    /// make response with exception
    pub fn exception(func: u8, code: Code) -> ResponsePdu {
        ResponsePdu::Exception {
            function: func | 0x80,
            code,
        }
    }

    /// raw
    pub fn raw(func: u8, data: Data) -> ResponsePdu {
        ResponsePdu::Raw {
            function: func,
            data,
        }
    }

    fn read_coils_inner(func: u8, coils: impl Coils) -> ResponsePdu {
        let nobjs = coils.coils_count();

        assert!(common::ncoils_check(nobjs));
        assert!(func == 0x1 || func == 0x2);

        match func {
            0x1 => ResponsePdu::ReadCoils {
                nobjs,
                data: Data::coils(coils),
            },
            0x2 => ResponsePdu::ReadDiscreteInputs {
                nobjs,
                data: Data::coils(coils),
            },
            _ => unreachable!(),
        }
    }

    fn read_registers_inner(func: u8, registers: impl Registers) -> ResponsePdu {
        let nobjs = registers.registers_count();
        assert!(common::nregs_check(nobjs));
        assert!(func == 0x3 || func == 0x4);

        match func {
            0x3 => ResponsePdu::ReadHoldingRegisters {
                nobjs,
                data: Data::registers(registers),
            },
            0x4 => ResponsePdu::ReadInputRegisters {
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
        let frame = ResponseFrame::new(0x11, ResponsePdu::read_coils(bits.as_slice()));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::ReadCoils { nobjs, data } => {
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
        let frame = ResponseFrame::new(0x11, ResponsePdu::read_coils(bits.as_slice()));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::ReadCoils { nobjs, data } => {
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
        let frame = ResponseFrame::new(0x11, ResponsePdu::read_discrete_inputs(bits.as_slice()));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::ReadDiscreteInputs { nobjs, data } => {
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
        let registers = [1u16, 2, 0xFFFF];
        let frame = ResponseFrame::new(
            0x11,
            ResponsePdu::read_holding_registers(registers.as_slice()),
        );
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::ReadHoldingRegisters { nobjs, data } => {
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
        let registers = [1u16, 2, 3, 0xFFFF];
        let frame = ResponseFrame::new(
            0x11,
            ResponsePdu::read_input_registers(registers.as_slice()),
        );
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::ReadInputRegisters { nobjs, data } => {
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
        let frame = ResponseFrame::new(0x11, ResponsePdu::write_single_coil(0x00AC, true));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::WriteSingleCoil { address, value } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(value, true);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc6_response() {
        let frame = ResponseFrame::new(0x11, ResponsePdu::write_single_register(0x00AC, 0x123));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::WriteSingleRegister { address, value } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(value, 0x123);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc15_response() {
        let frame = ResponseFrame::new(0x11, ResponsePdu::write_multiple_coils(0x00AC, 0x10));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::WriteMultipleCoils { address, nobjs } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(nobjs, 0x10);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_fc16_response() {
        let frame = ResponseFrame::new(0x11, ResponsePdu::write_multiple_registers(0x00AC, 0x11));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::WriteMultipleRegisters { address, nobjs } => {
                assert_eq!(address, 0x00AC);
                assert_eq!(nobjs, 0x11);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_exception_response() {
        let frame = ResponseFrame::new(0x11, ResponsePdu::exception(0x3, Code::IllegalFunction));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            ResponsePdu::Exception { function, code } => {
                assert_eq!(function, 0x83);
                assert_eq!(code, Code::IllegalFunction);
            }
            _ => unreachable!(),
        }
    }
}
