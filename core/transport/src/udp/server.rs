use std::io::Error;
use std::net::SocketAddr;

use frame::request::RequestFrame;
use frame::response::ResponseFrame;
use log::debug;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_util::udp::UdpFramed;

use futures::{SinkExt, StreamExt};

extern crate codec;
extern crate frame;

use crate::{settings::Settings, Handler, Request};
use codec::udp::UdpCodec;

pub struct UdpServer {
    io: UdpFramed<UdpCodec>,
    request_tx: mpsc::Sender<Request>,
    response_tx: mpsc::Sender<ResponseFrame>,
    response_rx: mpsc::Receiver<ResponseFrame>,

    //TODO:link with msg ids
    last_addr: Option<SocketAddr>,
}

impl UdpServer {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let address = settings.address.get();
        let socket = UdpSocket::bind(address).await?;
        let codec = UdpCodec::new(address);
        let io = UdpFramed::new(socket, codec);

        let (tx, rx) = mpsc::channel(settings.nmsg);
        let (response_tx, response_rx) = mpsc::channel(1);
        let server = UdpServer {
            io,
            request_tx: tx,
            response_tx,
            response_rx,
            last_addr: None,
        };

        let handler = Handler { request_rx: rx };
        server.spawn();
        Ok(handler)
    }

    pub fn spawn(mut self) {
        tokio::spawn(async move { while self.run().await {} });
    }

    async fn run(&mut self) -> bool {
        tokio::select! {
            request = self.io.next() => {
                match request {
                    Some(Ok((request, address))) => {
                        self.last_addr = Some(address);
                        debug!("{} {:?}", address, request);
                        self.start_request(request).await;
                    }
                    Some(Err(_)) => {
                        unreachable!()
                    }
                    None => {
                        // keep running...
                    }
                }
            },

            response = self.response_rx.recv() => {
                match response {
                    Some(response) => {
                        debug!("{} {:?}", self.last_addr.unwrap(), response);
                        let _ = self.io.send((response, self.last_addr.unwrap())).await;
                    }
                    None => {}
                }
            }
        };
        true
    }

    async fn start_request(&mut self, request: RequestFrame) {
        let request = Request {
            response_tx: self.response_tx.clone(),
            request,
        };
        let _ = self.request_tx.send(request).await;
    }
}
