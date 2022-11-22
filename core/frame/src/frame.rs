use super::pdu::{RequestPdu, ResponsePdu};

#[derive(Debug, PartialEq)]
pub struct RequestFrame {
    pub id: u16,
    pub slave: u8,
    pub pdu: RequestPdu,
}

impl RequestFrame {
    pub fn new(slave: u8, pdu: RequestPdu) -> RequestFrame {
        RequestFrame { id: 0, slave, pdu }
    }

    pub fn from_parts(id: u16, slave: u8, pdu: RequestPdu) -> RequestFrame {
        RequestFrame { id, slave, pdu }
    }
}

#[derive(Debug, PartialEq)]
pub struct ResponseFrame {
    pub id: u16,
    pub slave: u8,
    pub pdu: ResponsePdu,
}

impl ResponseFrame {
    pub fn new(slave: u8, pdu: ResponsePdu) -> ResponseFrame {
        ResponseFrame { id: 0, slave, pdu }
    }
    pub fn from_parts(id: u16, slave: u8, pdu: ResponsePdu) -> ResponseFrame {
        ResponseFrame { id, slave, pdu }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{common, exception::Code};

    #[test]
    fn create_frame() {
        let frame = RequestFrame::new(0x11, RequestPdu::read_coils(1, 1));
        assert_eq!(frame.slave, 0x11);
        match frame.pdu {
            RequestPdu::ReadCoils { .. } => {}
            _ => unreachable!(),
        }
    }

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
