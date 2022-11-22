extern crate codec;
extern crate frame;
use crate::{settings::Settings, Handler, Request, Response};
use codec::net::tcp::TcpCodec;
use frame::{RequestFrame, ResponseFrame};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::io::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use uuid::{self, Uuid};

struct MsgInfo {
    uuid: Uuid,
    mbid: u16,
}

pub struct TcpServer {
    listener: TcpListener,
    request_tx: mpsc::Sender<Request>,
}

struct Client {
    io: Framed<TcpStream, TcpCodec>,
    request_tx: mpsc::Sender<Request>,

    response_tx: mpsc::Sender<Response>,
    response_rx: mpsc::Receiver<Response>,
    address: String,
    wait_for: Option<MsgInfo>,
}

impl Client {
    fn spawn(mut self) {
        info!("{} connected", self.address);
        tokio::spawn(async move { while self.run().await {} });
    }

    async fn run(&mut self) -> bool {
        tokio::select! {
            request = self.io.next() => {
                match request {
                    Some(Ok(request)) => {
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
                        self.send_response(response).await;
                    }
                    None => {}
                }
            }
        };
        true
    }

    async fn send_response(&mut self, response: Response) {
        let resp_match = self
            .wait_for
            .as_ref()
            .map_or(false, |info| info.uuid == response.uuid);
        if resp_match {
            let info = self.wait_for.take().unwrap();

            debug!(
                "send response {} to {}: {:?}",
                response.uuid, self.address, response.payload
            );

            let response =
                ResponseFrame::from_parts(info.mbid, response.payload.slave, response.payload.pdu);

            let _ = self.io.send(response).await;
        } else {
            warn!("invalid/expired uuid:{}", response.uuid);
        }
    }

    async fn start_request(&mut self, request: RequestFrame) {
        let uuid = Uuid::new_v4();
        let mbid = request.id;
        let request = Request {
            uuid,
            payload: request,
            response_tx: Some(self.response_tx.clone()),
        };

        debug!(
            "recv request {} from {}: {:?}",
            uuid, self.address, request.payload
        );

        let _ = self.request_tx.send(request).await;
        if self.wait_for.is_some() {
            warn!("{} overflow", self.address);
        }
        self.wait_for = Some(MsgInfo { uuid, mbid });
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        info!("{} close", self.address);
    }
}

impl TcpServer {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let listener = TcpListener::bind(settings.address.get()).await?;
        let (tx, rx) = mpsc::channel(settings.nmsg);
        let server = TcpServer {
            listener,
            request_tx: tx,
        };
        let handler = Handler { request_rx: rx };
        server.spawn();
        Ok(handler)
    }

    pub fn spawn(mut self) {
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
        let client = Client {
            request_tx: self.request_tx.clone(),
            response_tx: tx,
            response_rx: rx,
            address: address.clone(),
            io: Framed::new(stream, TcpCodec::new(address.as_str())),
            wait_for: None,
        };
        client.spawn();
    }
}
