use crate::{MAX_DATA_SIZE, MAX_NCOILS, MAX_NREGS};

pub fn ncoils_len(nobjs: u16) -> usize {
    if nobjs > 0 {
        ((nobjs - 1) / 8 + 1) as usize
    } else {
        0
    }
}

pub fn nregs_len(nobjs: u16) -> usize {
    (nobjs * 2) as usize
}

pub fn ncoils_check(nobjs: u16) -> bool {
    nobjs > 0 && nobjs as usize <= MAX_NCOILS
}

pub fn nregs_check(nobjs: u16) -> bool {
    nobjs > 0 && nobjs as usize <= MAX_NREGS
}

pub fn data_bytes_check(nobjs: usize) -> bool {
    nobjs > 0 && nobjs <= MAX_DATA_SIZE
}

pub fn bits_from_bytes(bytes: &[u8], nbits: usize) -> Vec<bool> {
    let mut bits = Vec::new();
    for i in 0..nbits {
        bits.push(get_bit(bytes, i).unwrap());
    }
    bits
}

pub fn get_bit(buffer: &[u8], idx: usize) -> Option<bool> {
    if idx < buffer.len() * 8 {
        let byte_idx = idx / 8;
        let offset = idx % 8;
        Some(buffer[byte_idx] & (1 << offset) > 0)
    } else {
        None
    }
}
