use super::data::Data;
use super::{coils::CoilsStorage, common, registers::RegisterStorage};

#[derive(Debug, PartialEq)]
pub enum RequestPDU {
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
    Raw {
        function: u8,
        data: Data,
    },
}

#[derive(Debug, PartialEq)]
pub struct RequestFrame {
    pub id: Option<u16>,
    /// message id (only ModbusTCP)
    pub slave: u8,
    pub pdu: RequestPDU,
}

impl RequestFrame {
    pub fn rtu(slave: u8, pdu: RequestPDU) -> RequestFrame {
        RequestFrame {
            id: None,
            slave,
            pdu,
        }
    }

    pub fn net(id: u16, slave: u8, pdu: RequestPDU) -> RequestFrame {
        RequestFrame {
            id: Some(id),
            slave,
            pdu,
        }
    }
}

impl RequestPDU {
    /// 0x1
    pub fn read_coils(address: u16, nobjs: u16) -> RequestPDU {
        assert!(common::ncoils_check(nobjs));
        RequestPDU::ReadCoils { address, nobjs }
    }

    /// 0x2
    pub fn read_discrete_inputs(address: u16, nobjs: u16) -> RequestPDU {
        assert!(common::ncoils_check(nobjs));
        RequestPDU::ReadDiscreteInputs { address, nobjs }
    }

    /// 0x3
    pub fn read_holding_registers(address: u16, nobjs: u16) -> RequestPDU {
        assert!(common::nregs_check(nobjs));
        RequestPDU::ReadHoldingRegisters { address, nobjs }
    }

    /// 0x4
    pub fn read_input_registers(address: u16, nobjs: u16) -> RequestPDU {
        assert!(common::nregs_check(nobjs));
        RequestPDU::ReadInputRegisters { address, nobjs }
    }

    /// 0x5
    pub fn write_single_coil(address: u16, value: bool) -> RequestPDU {
        RequestPDU::WriteSingleCoil { address, value }
    }

    /// 0x6
    pub fn write_single_register(address: u16, value: u16) -> RequestPDU {
        RequestPDU::WriteSingleRegister { address, value }
    }

    /// 0xF
    pub fn write_multiple_coils(address: u16, coils: impl CoilsStorage) -> RequestPDU {
        let nobjs = coils.coils_count();
        assert!(common::ncoils_check(nobjs));
        RequestPDU::WriteMultipleCoils {
            address,
            nobjs,
            data: Data::coils(coils),
        }
    }

    /// 0x10
    pub fn write_multiple_registers(address: u16, registers: impl RegisterStorage) -> RequestPDU {
        let nobjs = registers.registers_count() as u16;
        assert!(common::nregs_check(nobjs));
        RequestPDU::WriteMultipleRegisters {
            address,
            nobjs,
            data: Data::registers(registers),
        }
    }

    /// Raw
    pub fn raw(func: u8, data: Data) -> RequestPDU {
        RequestPDU::Raw {
            function: func,
            data,
        }
    }
}
