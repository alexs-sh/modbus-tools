use std::convert::From;
use std::io;

#[derive(Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod test {
    use super::*;
    use std::io;

    #[test]
    fn from_io_error() {
        let err = io::Error::new(io::ErrorKind::InvalidData, "");
        assert_eq!(Error::from(err), Error::InvalidData);

        let err = io::Error::new(io::ErrorKind::UnexpectedEof, "");
        assert_eq!(Error::from(err), Error::BufferToSmall);

        let err = io::Error::new(io::ErrorKind::Other, "");
        assert_eq!(Error::from(err), Error::Other);
    }
}
