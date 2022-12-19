pub mod builder;
pub mod rtu;
pub mod settings;
pub mod tcp;
pub mod udp;

use frame::{RequestPdu, ResponsePdu};
use futures::Stream;
use std::fmt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

#[derive(Debug)]
pub struct Request {
    pub uuid: Uuid,
    pub slave: u8,
    pub pdu: RequestPdu,
    pub response_tx: Option<mpsc::Sender<Response>>,
}

#[derive(Debug)]
pub struct Response {
    pub uuid: Uuid,
    pub slave: u8,
    pub pdu: ResponsePdu,
    response_tx: Option<mpsc::Sender<Response>>,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} slave:{} pdu:{:?}", self.uuid, self.slave, self.pdu)
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} slave:{} pdu:{:?}", self.uuid, self.slave, self.pdu)
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

    pub async fn send(mut self) {
        self.response_tx.take().unwrap().send(self).await.unwrap();
    }

    pub fn try_send(mut self) {
        self.response_tx.take().unwrap().try_send(self).unwrap();
    }
}

//Transport handler
pub struct Handler {
    pub request_rx: mpsc::Receiver<Request>,
}

impl Handler {
    pub fn to_stream(self) -> impl Stream<Item = Request> {
        ReceiverStream::new(self.request_rx)
    }
}
