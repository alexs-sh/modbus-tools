use super::{MAX_DATA_SIZE, MAX_NCOILS, MAX_NREGS};
pub fn check_coils_count(nobjs: u16) -> bool {
    nobjs > 0 && nobjs as usize <= MAX_NCOILS
}

pub fn check_registers_count(nobjs: u16) -> bool {
    nobjs > 0 && nobjs as usize <= MAX_NREGS
}

pub fn checks_bytes_count(nobjs: usize) -> bool {
    nobjs > 0 && nobjs <= MAX_DATA_SIZE
}
