pub mod bytes;
pub mod checks;
pub mod coils;
pub mod helpers;
pub mod registers;
pub mod storage;

pub const MAX_PDU_SIZE: usize = 253; // Max. size of  protocol data unit
pub const MAX_NREGS: usize = 125; // Max. number of registers
pub const MAX_NCOILS: usize = MAX_NREGS * 16; // Max. number of coils
pub const MAX_DATA_SIZE: usize = 256; // used for storing data in internal structs. Should has length that divides by 2

pub mod prelude {

    pub use super::bytes::{Bytes, BytesCursor};
    pub use super::coils::{Coils, CoilsCursor};
    pub use super::registers::{Registers, RegistersCursorBe};
    pub use super::storage::DataStorage as Data;
    pub use super::MAX_DATA_SIZE;
    pub use super::MAX_NCOILS;
    pub use super::MAX_NREGS;
    pub use super::MAX_PDU_SIZE; // Max. size of  protocol data unit
}
