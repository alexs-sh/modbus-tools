use super::{coils::Coils, common, registers::Registers, MAX_DATA_SIZE};

use smallvec::SmallVec;

#[derive(Debug, PartialEq)]
pub struct Data {
    buffer: SmallVec<[u8; MAX_DATA_SIZE]>,
}

impl Data {
    pub fn raw(bytes: &[u8]) -> Data {
        assert!(bytes.len() <= MAX_DATA_SIZE);
        let buffer = SmallVec::<[u8; MAX_DATA_SIZE]>::from(bytes);
        Data { buffer }
    }

    pub fn raw_empty(size: usize) -> Data {
        assert!(size <= MAX_DATA_SIZE);
        let mut buffer = SmallVec::<[u8; MAX_DATA_SIZE]>::new();
        buffer.resize(size, 0);
        Data { buffer }
    }

    pub fn coils(coils: impl Coils) -> Data {
        let nobjs = coils.coils_count();
        let mut data = Data::coils_empty(nobjs);
        let written = coils.coils_write(data.get_mut());
        assert!(written == nobjs);
        data
    }

    pub fn registers(registers: impl Registers) -> Data {
        let nobjs = registers.registers_count();
        let mut data = Data::registers_empty(nobjs);
        let written = registers.registers_write(data.get_mut());
        assert!(written == nobjs);
        data
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn get(&self) -> &[u8] {
        let len = self.len();
        &self.buffer[0..len]
    }

    pub fn extend(&mut self, slice: &[u8]) {
        self.buffer.extend_from_slice(slice);
    }

    pub fn get_mut(&mut self) -> &mut [u8] {
        let len = self.len();
        &mut self.buffer[0..len]
    }

    pub fn get_u8(&self, idx: usize) -> Option<u8> {
        if idx < self.len() {
            Some(self.get()[idx])
        } else {
            None
        }
    }

    pub fn set_u8(&mut self, idx: usize, value: u8) -> bool {
        if idx < self.len() {
            self.get_mut()[idx] = value;
            true
        } else {
            false
        }
    }

    pub fn get_bit(&self, idx: usize) -> Option<bool> {
        common::get_bit(self.get(), idx)
    }

    pub fn set_bit(&mut self, idx: usize, value: bool) -> bool {
        assert!(idx < self.len() * 8);

        let byte_idx = idx / 8;
        let offset = idx % 8;
        if value {
            self.get_mut()[byte_idx] |= 1 << offset;
        } else {
            self.get_mut()[byte_idx] &= !(1 << offset);
        }
        true
    }

    pub fn set_u16(&mut self, idx: usize, value: u16) -> bool {
        let start = idx * 2;
        let end = start + 1;
        assert!(end < self.len());
        self.get_mut()[start..end + 1].copy_from_slice(&value.to_ne_bytes());
        true
    }

    pub fn get_u16(&self, idx: usize) -> Option<u16> {
        let start = idx * 2;
        let end = start + 1;

        if end < self.len() {
            Some(u16::from_ne_bytes(
                self.get()[start..end + 1].try_into().unwrap(),
            ))
        } else {
            None
        }
    }

    fn registers_empty(nobjs: u16) -> Data {
        assert!(common::nregs_check(nobjs as u16));

        let len = common::nregs_len(nobjs);
        let mut buffer = SmallVec::<[u8; MAX_DATA_SIZE]>::new();
        buffer.resize(len as usize, 0);
        Data { buffer }
    }

    fn coils_empty(nobjs: u16) -> Data {
        assert!(common::ncoils_check(nobjs));

        let len = common::ncoils_len(nobjs);
        let mut buffer = SmallVec::<[u8; MAX_DATA_SIZE]>::new();
        buffer.resize(len as usize, 0);
        Data { buffer }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn data_coils() {
        let input = [true, false, false, false, true, false, false, false];

        let data = Data::coils(&input[0..1]);
        assert_eq!(data.len(), 1);
        assert_eq!(data.get_bit(0).unwrap(), true);
        assert_eq!(data.get_u8(0).unwrap(), 0x1);
        assert!(data.get_u16(0).is_none());

        let data = Data::coils(&input[..]);
        assert_eq!(data.len(), 1);
        assert_eq!(data.get_bit(0).unwrap(), true);
        assert_eq!(data.get_u8(0).unwrap(), 0x1 | 0x10);
        assert!(data.get_u16(0).is_none());
    }

    #[test]
    fn data_registers() {
        let input = [1u16, 2, 3, 4];
        let data = Data::registers(&input[0..1]);
        assert_eq!(data.len(), 2);
        assert_eq!(data.get_u16(0).unwrap(), 0x1);
        assert!(data.get_u16(1).is_none());

        let data = Data::registers(&input[..]);
        assert_eq!(data.len(), 8);
        assert_eq!(data.get_u16(0).unwrap(), 0x1);
        assert_eq!(data.get_u16(1).unwrap(), 0x2);
        assert_eq!(data.get_u16(2).unwrap(), 0x3);
        assert_eq!(data.get_u16(3).unwrap(), 0x4);
    }

    #[test]
    fn data_raw() {
        let input = [1u8, 2, 3, 4];
        let data = Data::raw(&input);
        assert_eq!(data.len(), 4);
        assert_eq!(data.get_u8(0).unwrap(), 0x1);
        assert_eq!(data.get_u8(1).unwrap(), 0x2);
        assert!(data.get_u8(4).is_none());
    }

    #[test]
    fn data_ops() {
        let input = [1u8, 2, 3, 4];
        let mut data = Data::raw(&input);
        assert_eq!(data.len(), 4);
        assert_eq!(data.get_u8(0).unwrap(), 0x1);

        data.set_u8(0, 0xAA);
        assert_eq!(data.get_u8(0).unwrap(), 0xAA);

        data.set_u8(1, 0xBB);
        assert_eq!(data.get_u8(1).unwrap(), 0xBB);
        assert_eq!(data.get_u16(0).unwrap(), 0xBBAA);

        assert_eq!(data.get_bit(0).unwrap(), false);
        assert_eq!(data.get_bit(1).unwrap(), true);

        data.set_bit(0, true);
        data.set_bit(1, false);
        assert_eq!(data.get_bit(0).unwrap(), true);
        assert_eq!(data.get_bit(1).unwrap(), false);
    }
}
