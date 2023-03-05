pub mod builder;
pub mod context;
pub mod event;
pub mod rtu;
pub mod settings;
pub mod tcp;
pub mod udp;

use crate::frame::prelude::*;

use futures::Stream;
use std::fmt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

#[derive(Debug)]
pub struct Request {
    pub uuid: Uuid,
    pub slave: u8,
    pub pdu: RequestPdu,
    pub response_tx: Option<mpsc::UnboundedSender<Response>>,
}

#[derive(Debug)]
pub struct Response {
    pub uuid: Uuid,
    pub slave: u8,
    pub pdu: ResponsePdu,
    response_tx: Option<mpsc::UnboundedSender<Response>>,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "response id:{} slave:{} pdu:{:?}",
            self.uuid, self.slave, self.pdu
        )
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "request id:{} slave:{} pdu:{:?}",
            self.uuid, self.slave, self.pdu
        )
    }
}

impl Response {
    pub fn make(mut request: Request, response: ResponsePdu) -> Response {
        Response {
            uuid: request.uuid,
            slave: request.slave,
            pdu: response,
            response_tx: request.response_tx.take(),
        }
    }

    pub fn send(mut self) {
        self.response_tx.take().unwrap().send(self).unwrap();
    }
}

pub struct Handler {
    pub request_rx: mpsc::UnboundedReceiver<Request>,
}

impl Handler {
    pub fn to_stream(self) -> impl Stream<Item = Request> {
        UnboundedReceiverStream::new(self.request_rx)
    }
}

pub mod prelude {
    pub use super::context::IoContext;
    pub use super::settings::{Settings, TransportAddress};
    pub use super::Handler;
    pub use super::Request;
    pub use super::Response;
}
