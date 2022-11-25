extern crate codec;
extern crate frame;

use super::queue::FixedQueue;
use crate::{settings::Settings, Handler, Request, Response};
use codec::helpers;
use codec::net::udp::UdpCodec;
use frame::{RequestFrame, ResponseFrame};
use futures::{SinkExt, StreamExt};
use log::warn;
use std::io::Error;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_util::udp::UdpFramed;
use uuid::{self, Uuid};

struct MsgInfo {
    uuid: Uuid,
    mbid: u16,
    address: SocketAddr,
}

pub struct UdpServer {
    io: UdpFramed<UdpCodec>,
    request_tx: mpsc::Sender<Request>,
    response_tx: mpsc::Sender<Response>,
    response_rx: mpsc::Receiver<Response>,
    queue: FixedQueue<MsgInfo>,
}

impl UdpServer {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let address = settings.address.get();
        let socket = UdpSocket::bind(address).await?;
        let codec = UdpCodec::new("udp");
        let io = UdpFramed::new(socket, codec);

        let (tx, rx) = mpsc::channel(settings.nmsg);
        let (response_tx, response_rx) = mpsc::channel(1);
        let server = UdpServer {
            io,
            request_tx: tx,
            response_tx,
            response_rx,
            queue: FixedQueue::new(settings.nmsg),
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
                        self.start_request(request, address).await;
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
                if let Some(response) = response {
                    self.send_response(response).await;
                }
            }
        };
        true
    }

    async fn start_request(&mut self, request: RequestFrame, address: SocketAddr) {
        let uuid = Uuid::new_v4();
        let info = MsgInfo {
            uuid,
            mbid: request.id,
            address,
        };
        self.queue.push_replace(info);

        let request = Request {
            uuid,
            payload: request,
            response_tx: Some(self.response_tx.clone()),
        };
        helpers::log_frame(&address, &uuid, &request.payload);
        let _ = self.request_tx.send(request).await;
    }

    async fn send_response(&mut self, response: Response) {
        if let Some(info) = self.queue.take_if(|rec| rec.uuid == response.uuid) {
            helpers::log_frame(&info.address, &response.uuid, &response.payload);
            let id = info.mbid;
            let response =
                ResponseFrame::from_parts(id, response.payload.slave, response.payload.pdu);
            let _ = self.io.send((response, info.address)).await;
        } else {
            warn!("invalid/expired uuid:{}", response.uuid);
        }
    }
}
