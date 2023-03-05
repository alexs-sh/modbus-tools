pub fn get_coils_len(nobjs: u16) -> usize {
    if nobjs > 0 {
        ((nobjs - 1) / 8 + 1) as usize
    } else {
        0
    }
}

pub fn get_registers_len(nobjs: u16) -> usize {
    (nobjs * 2) as usize
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
