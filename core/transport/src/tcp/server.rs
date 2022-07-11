use std::io::Error;
use std::net::SocketAddr;

use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

use frame::request::RequestFrame;
use frame::response::ResponseFrame;

use futures::{SinkExt, StreamExt};

extern crate codec;
extern crate frame;

use codec::net::default::Codec as NetCodec;

pub struct TcpServerHandler {
    pub request_rx: mpsc::Receiver<Request>,
}

pub struct Request {
    response_tx: mpsc::Sender<ResponseFrame>,
    pub request: RequestFrame,
}

#[derive(Clone)]
pub struct TcpServerSettings {
    pub address: String,
    pub nmsg: usize,
}

impl Request {
    pub async fn response(&self, mut response: frame::response::ResponseFrame) {
        response.id = self.request.id;
        self.response_tx.send(response).await.unwrap();
    }
}

pub struct TcpServer {
    listener: TcpListener,
    request_tx: mpsc::Sender<Request>,
}

struct TcpClient {
    io: Framed<TcpStream, codec::net::default::Codec>,
    request_tx: mpsc::Sender<Request>,
    response_tx: mpsc::Sender<ResponseFrame>,
    response_rx: mpsc::Receiver<ResponseFrame>,
    address: String,
}

impl TcpClient {
    fn spawn(mut self) {
        info!("{} connected", self.address);
        tokio::spawn(async move { while self.run().await {} });
    }

    async fn run(&mut self) -> bool {
        tokio::select! {
            request = self.io.next() => {
                match request {
                    Some(Ok(request)) => {
                        debug!("{} {:?}", self.address, request);
                        self.start_request(request).await;
                    }
                    Some(Err(info)) => {
                        error!("{} parser error: {:?}", self.address, info);
                        return false;
                    }
                    None => {
                        return false;
                    }
                }
            },

            response = self.response_rx.recv() => {
                match response {
                    Some(response) => {
                        debug!("{} {:?}", self.address, response);
                        let _ = self.io.send(response).await;
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

impl Drop for TcpClient {
    fn drop(&mut self) {
        info!("{} close", self.address);
    }
}

impl Default for TcpServerSettings {
    fn default() -> TcpServerSettings {
        TcpServerSettings {
            address: "0.0.0.0:502".to_owned(),
            nmsg: 128,
        }
    }
}

impl TcpServer {
    pub async fn build(settings: TcpServerSettings) -> Result<TcpServerHandler, Error> {
        let listener = TcpListener::bind(settings.address).await?;
        let (tx, rx) = mpsc::channel(settings.nmsg);
        let server = TcpServer {
            listener,
            request_tx: tx,
        };
        let handler = TcpServerHandler { request_rx: rx };
        server.spawn();
        Ok(handler)
    }

    fn spawn(mut self) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok((stream,address)) = self.listener.accept() => {
                        self.spawn_client(stream, address);
                    }
                }
            }
        });
    }

    fn spawn_client(&mut self, stream: TcpStream, address: SocketAddr) {
        let (tx, rx) = mpsc::channel(1);
        let address = address.to_string();
        let client = TcpClient {
            request_tx: self.request_tx.clone(),
            response_tx: tx,
            response_rx: rx,
            address: address.clone(),
            io: Framed::new(stream, NetCodec::new(address)),
        };
        client.spawn();
    }
}
