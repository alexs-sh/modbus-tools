use super::common;
use super::data::{Bytes, Coils, Data, Registers};

#[derive(Debug, PartialEq)]
pub enum RequestPdu {
    /// 0x1
    ReadCoils {
        address: u16,
        nobjs: u16,
    },

    /// 0x2
    ReadDiscreteInputs {
        address: u16,
        nobjs: u16,
    },

    /// 0x3
    ReadHoldingRegisters {
        address: u16,
        nobjs: u16,
    },

    /// 0x4
    ReadInputRegisters {
        address: u16,
        nobjs: u16,
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
        data: Data,
    },

    /// 0x10
    WriteMultipleRegisters {
        address: u16,
        nobjs: u16,
        data: Data,
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
}

#[derive(Debug, PartialEq)]
pub struct RequestFrame {
    pub slave: u8,
    pub pdu: RequestPdu,
}

impl RequestFrame {
    pub fn new(slave: u8, pdu: RequestPdu) -> RequestFrame {
        RequestFrame { slave, pdu }
    }
}

impl RequestPdu {
    /// 0x1
    pub fn read_coils(address: u16, nobjs: u16) -> RequestPdu {
        assert!(common::ncoils_check(nobjs));
        RequestPdu::ReadCoils { address, nobjs }
    }

    /// 0x2
    pub fn read_discrete_inputs(address: u16, nobjs: u16) -> RequestPdu {
        assert!(common::ncoils_check(nobjs));
        RequestPdu::ReadDiscreteInputs { address, nobjs }
    }

    /// 0x3
    pub fn read_holding_registers(address: u16, nobjs: u16) -> RequestPdu {
        assert!(common::nregs_check(nobjs));
        RequestPdu::ReadHoldingRegisters { address, nobjs }
    }

    /// 0x4
    pub fn read_input_registers(address: u16, nobjs: u16) -> RequestPdu {
        assert!(common::nregs_check(nobjs));
        RequestPdu::ReadInputRegisters { address, nobjs }
    }

    /// 0x5
    pub fn write_single_coil(address: u16, value: bool) -> RequestPdu {
        RequestPdu::WriteSingleCoil { address, value }
    }

    /// 0x6
    pub fn write_single_register(address: u16, value: u16) -> RequestPdu {
        RequestPdu::WriteSingleRegister { address, value }
    }

    /// 0xF
    pub fn write_multiple_coils(address: u16, coils: impl Coils) -> RequestPdu {
        let nobjs = coils.coils_count();
        assert!(common::ncoils_check(nobjs));
        RequestPdu::WriteMultipleCoils {
            address,
            nobjs,
            data: Data::coils(coils),
        }
    }

    /// 0x10
    pub fn write_multiple_registers(address: u16, registers: impl Registers) -> RequestPdu {
        let nobjs = registers.registers_count() as u16;
        assert!(common::nregs_check(nobjs));
        RequestPdu::WriteMultipleRegisters {
            address,
            nobjs,
            data: Data::registers(registers),
        }
    }

    /// 0x2b
    pub fn encapsulated_interface_transport(mei_type: u8, bytes: impl Bytes) -> RequestPdu {
        let len = bytes.bytes_count() as usize;

        assert!(common::data_bytes_check(len));

        let mut data = Data::raw_empty(len);
        bytes.bytes_write(data.get_mut());

        RequestPdu::EncapsulatedInterfaceTransport { mei_type, data }
    }

    /// Raw
    pub fn raw(func: u8, data: Data) -> RequestPdu {
        RequestPdu::Raw {
            function: func,
            data,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_frame() {
        let frame = RequestFrame::new(0x11, RequestPdu::read_coils(1, 1));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            RequestPdu::ReadCoils { .. } => {}
            _ => unreachable!(),
        }
    }
}
