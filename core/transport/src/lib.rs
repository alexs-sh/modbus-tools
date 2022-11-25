pub mod builder;
pub mod rtu;
pub mod settings;
pub mod tcp;
pub mod udp;

use frame::{RequestFrame, ResponseFrame};
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

#[derive(Debug)]
pub struct Request {
    pub uuid: Uuid,
    pub payload: RequestFrame,
    pub response_tx: Option<mpsc::Sender<Response>>,
}

#[derive(Debug)]
pub struct Response {
    pub uuid: Uuid,
    pub payload: ResponseFrame,
    response_tx: Option<mpsc::Sender<Response>>,
}

impl Response {
    pub fn make(mut request: Request, response: ResponseFrame) -> Response {
        Response {
            uuid: request.uuid,
            payload: response,
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
