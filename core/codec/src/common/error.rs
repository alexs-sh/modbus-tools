use std::convert::From;
use std::io;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidData,
    InvalidVersion,
    BufferToSmall,
    Other,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::InvalidData => Error::InvalidData,
            io::ErrorKind::UnexpectedEof => Error::BufferToSmall,
            _ => Error::Other,
        }
    }
}
