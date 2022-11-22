pub mod common;
pub mod data;
pub mod exception;
mod frame;
mod pdu;

pub const MAX_PDU_SIZE: usize = 253; // Max. size of  protocol data unit
pub const MAX_NREGS: usize = 125; // Max. number of registers
pub const MAX_NCOILS: usize = MAX_NREGS * 16; // Max. number of coils
pub const MAX_DATA_SIZE: usize = 256; // used for storing data in internal structs. Should has length that divides by 2
pub const MBAP_HEADER_LEN: usize = 7;

pub const COIL_ON: u16 = 0xFF00;
pub const COIL_OFF: u16 = 0x0000;

pub use crate::frame::RequestFrame;
pub use crate::frame::ResponseFrame;
pub use crate::pdu::RequestPdu;
pub use crate::pdu::ResponsePdu;
