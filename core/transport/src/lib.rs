pub mod builder;
pub mod settings;
pub mod tcp;
pub mod udp;

use frame::request::RequestFrame;
use frame::response::ResponseFrame;
use tokio::sync::mpsc;

use futures::Stream;
use tokio_stream::wrappers::ReceiverStream;

pub struct Request {
    response_tx: mpsc::Sender<ResponseFrame>,
    pub request: RequestFrame,
}

impl Request {
    pub async fn response(&self, mut response: frame::response::ResponseFrame) {
        response.id = self.request.id;
        self.response_tx.send(response).await.unwrap();
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
