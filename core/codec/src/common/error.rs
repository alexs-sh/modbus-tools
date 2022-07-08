use std::convert::From;
use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq)]
pub enum CodecError {
    InvalidData,
    InvalidVersion,
    UnsupportedFunction,
    BufferToSmall,
    Other,
}

impl From<Error> for CodecError {
    fn from(error: Error) -> Self {
        match error.kind() {
            ErrorKind::InvalidData => CodecError::InvalidData,
            ErrorKind::Unsupported => CodecError::UnsupportedFunction,
            ErrorKind::UnexpectedEof => CodecError::BufferToSmall,
            _ => CodecError::Other,
        }
    }
}
