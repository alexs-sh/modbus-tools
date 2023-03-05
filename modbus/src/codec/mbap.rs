use crate::codec::context::{ReadCtx, WriteCtx};
use crate::codec::error::Error;
use crate::codec::wait;
use crate::data::MAX_DATA_SIZE;

use crate::frame::prelude::*;

pub(crate) struct Mbap {
    pub id: u16,
    pub proto: u16,
    pub len: u16,
    pub slave: u8,
}

pub(crate) fn read_mbap(ctx: &mut ReadCtx) -> Result<Option<Mbap>, Error> {
    let id = wait!(ctx.read_u16_be());
    let proto = wait!(ctx.read_u16_be());
    let len = wait!(ctx.read_u16_be());
    let slave = wait!(ctx.read_u8());
    let mbap = Mbap {
        id,
        proto,
        len,
        slave,
    };

    validate_mbap(&mbap)?;
    Ok(Some(mbap))
}

pub(crate) fn write_mbap(ctx: &mut WriteCtx, frame: &ResponseFrame) -> Result<(), Error> {
    ctx.write_u16_be(frame.id).unwrap();
    ctx.write_u16_be(0).unwrap();
    ctx.write_u16_be(frame.pdu.len() as u16 + 1).unwrap();
    Ok(())
}

fn validate_mbap(mbap: &Mbap) -> Result<(), Error> {
    if mbap.proto != 0 {
        Err(Error::InvalidVersion)
    } else if mbap.len < 2 || mbap.len as usize > MAX_DATA_SIZE {
        Err(Error::InvalidData)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{read_mbap, ReadCtx};

    #[test]
    fn read_net_mbap() {
        let buffer = [0x0, 0x1, 0x0, 0x0, 0x0, 0x6, 0x11];
        let mbap = read_mbap(&mut ReadCtx::new(&buffer)).unwrap().unwrap();
        assert_eq!(mbap.id, 0x1);
        assert_eq!(mbap.proto, 0x0);
        assert_eq!(mbap.len, 0x6);
        assert_eq!(mbap.slave, 0x11);
    }

    #[test]
    fn read_net_mbap_invalid() {
        let check = [
            vec![0x0, 0x1, 0x0, 0x1, 0x0, 0x6, 0x11],
            vec![0x0, 0x1, 0x0, 0x0, 0xFF, 0x6, 0x11],
        ];

        for rec in check {
            let mbap = read_mbap(&mut ReadCtx::new(&rec));
            match mbap {
                Err(_) => {}
                _ => unreachable!(),
            }
        }
    }
}
