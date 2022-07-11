extern crate frame;

use crate::common::error::Error;

use frame::{data::Data, response::ResponsePDU, COIL_OFF, COIL_ON};

use byteorder::{BigEndian, WriteBytesExt};
use bytes::Buf;
use std::io::Cursor;

pub(crate) fn pack_response(src: &ResponsePDU, dst: &mut Cursor<&mut [u8]>) -> Result<(), Error> {
    match src {
        ResponsePDU::ReadCoils { data, .. } => {
            check_capacity(data.len() + 2, dst)?;
            dst.write_u8(0x1)?;
            dst.write_u8(data.len() as u8)?;
            write_coils_data(data, dst);
            Ok(())
        }
        ResponsePDU::ReadDiscreteInputs { data, .. } => {
            check_capacity(data.len() + 2, dst)?;
            dst.write_u8(0x2)?;
            dst.write_u8(data.len() as u8)?;
            write_coils_data(data, dst);
            Ok(())
        }
        ResponsePDU::ReadHoldingRegisters { data, .. } => {
            check_capacity(data.len() + 2, dst)?;
            dst.write_u8(0x3)?;
            dst.write_u8(data.len() as u8)?;
            write_regs_data(data, dst);
            Ok(())
        }
        ResponsePDU::ReadInputRegisters { data, .. } => {
            check_capacity(data.len() + 2, dst)?;
            dst.write_u8(0x4)?;
            dst.write_u8(data.len() as u8)?;
            write_regs_data(data, dst);
            Ok(())
        }
        ResponsePDU::WriteSingleCoil { address, value } => {
            check_capacity(5, dst)?;
            dst.write_u8(0x5)?;
            dst.write_u16::<BigEndian>(*address)?;
            dst.write_u16::<BigEndian>(if *value { COIL_ON } else { COIL_OFF })?;
            Ok(())
        }
        ResponsePDU::WriteSingleRegister { address, value } => {
            check_capacity(5, dst)?;
            dst.write_u8(0x6)?;
            dst.write_u16::<BigEndian>(*address)?;
            dst.write_u16::<BigEndian>(*value)?;
            Ok(())
        }

        ResponsePDU::WriteMultipleCoils { address, nobjs } => {
            check_capacity(5, dst)?;
            dst.write_u8(0xF)?;
            dst.write_u16::<BigEndian>(*address)?;
            dst.write_u16::<BigEndian>(*nobjs)?;
            Ok(())
        }
        ResponsePDU::WriteMultipleRegisters { address, nobjs } => {
            check_capacity(5, dst)?;
            dst.write_u8(0x10)?;
            dst.write_u16::<BigEndian>(*address)?;
            dst.write_u16::<BigEndian>(*nobjs)?;
            Ok(())
        }
        ResponsePDU::Exception { function, code } => {
            check_capacity(2, dst)?;
            dst.write_u8(*function)?;
            dst.write_u8((*code) as u8)?;
            Ok(())
        }
        _ => unreachable!(),
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
        let pdu = ResponsePDU::read_coils(&bits);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }

    #[test]
    fn pack_fc3() {
        let regs = [0xAE41, 0x5652, 0x4340];
        let control = [0x03u8, 0x06, 0xAE, 0x41, 0x56, 0x52, 0x43, 0x40];
        let pdu = ResponsePDU::read_holding_registers(&regs);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }

    #[test]
    fn pack_fc4() {
        let regs = [0x000A];
        let control = [0x04, 0x02, 0x00, 0x0A];
        let pdu = ResponsePDU::read_input_registers(&regs);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }

    #[test]
    fn pack_fc5() {
        let control = [0x05, 0x00, 0xAC, 0xFF, 0x00];
        let pdu = ResponsePDU::write_single_coil(0x00AC, true);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }

    #[test]
    fn pack_fc6() {
        let control = [0x06, 0x00, 0x01, 0x00, 0x03];
        let pdu = ResponsePDU::write_single_register(0x0001, 0x0003);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }

    #[test]
    fn pack_fc15() {
        let control = [0x0F, 0x00, 0x13, 0x00, 0x0A];
        let pdu = ResponsePDU::write_multiple_coils(0x0013, 0x000A);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }

    #[test]
    fn pack_fc16() {
        let control = [0x10, 0x00, 0x01, 0x00, 0x02];
        let pdu = ResponsePDU::write_multiple_registers(0x0001, 0x0002);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }

    #[test]
    fn pack_exception() {
        let control = [0x81, 0x02];
        let pdu = ResponsePDU::exception(0x1, Code::IllegalDataAddress);
        let mut buffer = [0u8; 256];
        let pos = {
            let mut cursor = Cursor::new(&mut buffer[..]);
            pack_response(&pdu, &mut cursor).unwrap();
            cursor.position() as usize
        };
        assert_eq!(&control[..], &buffer[0..pos]);
    }
}
