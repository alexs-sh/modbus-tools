use crate::codec::error::Error as MbError;
use crate::codec::slave::SlaveCodec;
use crate::frame::prelude::*;
use bytes::BytesMut;
use std::io::{Error, ErrorKind};
use tokio_util::codec::{Decoder, Encoder};

pub struct IoContext {
    pub codec: SlaveCodec,
    pub input: BytesMut,
    pub output: BytesMut,
}

impl IoContext {
    pub fn new(codec: SlaveCodec) -> IoContext {
        IoContext {
            codec,
            input: BytesMut::new(),
            output: BytesMut::new(),
        }
    }

    pub fn decode(&mut self) -> Result<Option<RequestFrame>, Error> {
        self.codec.decode(&mut self.input).map_err(|err| match err {
            MbError::InvalidCrc => Error::new(ErrorKind::InvalidData, "bad CRC"),
            _ => Error::new(ErrorKind::InvalidData, "bad input"),
        })
    }

    pub fn encode(&mut self, response: ResponseFrame) -> Result<(), Error> {
        self.codec
            .encode(response, &mut self.output)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "codec error"))
    }

    pub fn reset(&mut self) {
        self.input.clear();
        self.output.clear();
    }

    pub fn resize_input(&mut self, size: usize) {
        self.input.resize(size, 0);
    }
}
