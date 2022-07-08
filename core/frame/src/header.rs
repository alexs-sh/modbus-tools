use super::MAX_DATA_SIZE;

#[derive(Debug)]
pub struct Header {
    pub id: u16,
    pub proto: u16,
    pub len: u16,
    pub slave: u8,
}

impl Header {
    pub fn new(id: u16, len: u16, slave: u8) -> Header {
        assert!(len > 0);
        assert!(len < MAX_DATA_SIZE as u16);
        Header {
            id,
            proto: 0,
            len,
            slave,
        }
    }
}
