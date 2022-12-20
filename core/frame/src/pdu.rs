use super::data::{Bytes, Coils, Data, Registers};
use super::{common, exception::Code};

#[derive(Debug, PartialEq, Eq)]
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
        let nobjs = registers.registers_count();
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

    pub fn len(&self) -> usize {
        match self {
            RequestPdu::ReadCoils { .. }
            | RequestPdu::ReadDiscreteInputs { .. }
            | RequestPdu::ReadHoldingRegisters { .. }
            | RequestPdu::ReadInputRegisters { .. }
            | RequestPdu::WriteSingleCoil { .. }
            | RequestPdu::WriteSingleRegister { .. } => 5,

            RequestPdu::WriteMultipleCoils { data, .. }
            | RequestPdu::WriteMultipleRegisters { data, .. } => 6 + data.len(),

            RequestPdu::EncapsulatedInterfaceTransport { data, .. } => 2 + data.len(),
            RequestPdu::Raw { data, .. } => 1 + data.len(),
        }
    }

    pub fn func(&self) -> Option<u8> {
        match self {
            RequestPdu::ReadCoils { .. } => Some(0x1),
            RequestPdu::ReadDiscreteInputs { .. } => Some(0x2),
            RequestPdu::ReadHoldingRegisters { .. } => Some(0x3),
            RequestPdu::ReadInputRegisters { .. } => Some(0x4),
            RequestPdu::WriteSingleCoil { .. } => Some(0x5),
            RequestPdu::WriteSingleRegister { .. } => Some(0x6),
            RequestPdu::WriteMultipleCoils { .. } => Some(0xF),
            RequestPdu::WriteMultipleRegisters { .. } => Some(0x10),
            RequestPdu::EncapsulatedInterfaceTransport { .. } => Some(0x2b),
            RequestPdu::Raw { function, .. } => Some(*function),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
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
