use crate::codec::slave::SlaveCodec;
use crate::frame::prelude::*;
use crate::transport::{event::EventLog, prelude::*, udp::queue::FixedQueue};
use std::io::Error;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use uuid::{self, Uuid};

const MAX_BUFFER_SIZE: usize = 512;
const MAX_REQUESTS_NUM: usize = 256;

struct MsgInfo {
    uuid: Uuid,
    mbid: u16,
    address: SocketAddr,
}

pub struct UdpServer {
    socket: UdpSocket,
    context: IoContext,
    request_tx: mpsc::UnboundedSender<Request>,
    response_tx: mpsc::UnboundedSender<Response>,
    response_rx: mpsc::UnboundedReceiver<Response>,
    queue: FixedQueue<MsgInfo>,
}

impl UdpServer {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let address = settings.address.get();
        let codec = SlaveCodec::new_udp();
        let context = IoContext::new(codec);
        let socket = UdpSocket::bind(address).await?;
        let (tx, rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        let server = UdpServer {
            socket,
            context,
            request_tx: tx,
            response_tx,
            response_rx,
            queue: FixedQueue::new(MAX_REQUESTS_NUM),
        };

        let handler = Handler { request_rx: rx };
        server.spawn();
        Ok(handler)
    }

    pub fn spawn(mut self) {
        tokio::spawn(async move {
            loop {
                let _ = self.run().await;
            }
        });
    }

    async fn run(&mut self) -> Result<(), Error> {
        self.context.resize_input(MAX_BUFFER_SIZE);

        let read = self
            .socket
            .recv_from(&mut self.context.input[..MAX_BUFFER_SIZE]);

        tokio::select! {
            result = read => {
                match result {
                    Ok((0, _)) => {
                        // no data
                        Ok(())
                    },
                    Ok((size, address)) => {
                        self.context.resize_input(size);
                        self.on_input(address).await.map_err(|err|
                            {
                                EventLog::error(&address,&err);
                                err
                            })
                    }
                    Err(err) => {
                        EventLog::error(&"UDP server",&err);
                        Err(err)
                    }
                }
            },

            response = self.response_rx.recv() => {
                self.on_response(response).await
            }
        }
    }

    async fn on_input(&mut self, address: SocketAddr) -> Result<(), Error> {
        EventLog::input(&address, &self.context.input);
        let Some(request) = self.context.decode()? else {
            return Ok(());
        };
        self.on_request(address, request).await;
        Ok(())
    }

    async fn on_request(&mut self, address: SocketAddr, request: RequestFrame) {
        let uuid = Uuid::new_v4();
        let info = MsgInfo {
            uuid,
            mbid: request.id,
            address,
        };

        let request = Request {
            uuid,
            slave: request.slave,
            pdu: request.pdu,
            response_tx: Some(self.response_tx.clone()),
        };

        EventLog::request(&address, &request);

        if self.request_tx.send(request).is_ok() {
            self.queue.push_replace(info);
        } else {
            EventLog::warning(&address, &"can't process input request.TX overflow?");
        }
    }

    async fn on_response(&mut self, response: Option<Response>) -> Result<(), Error> {
        let Some(response) = response else {
            return Ok(());
        };
        let Some(info) = self.queue.take_if(|rec| rec.uuid == response.uuid) else {
            EventLog::warning(&response.uuid, &"uuid is missing/expired");
            return Ok(());
        };

        EventLog::response(&info.address, &response);
        let frame = ResponseFrame::from_parts(info.mbid, response.slave, response.pdu);
        self.on_output(info.address, frame).await.map(|_| ())
    }

    async fn on_output(
        &mut self,
        address: SocketAddr,
        frame: ResponseFrame,
    ) -> Result<usize, Error> {
        self.context.encode(frame)?;
        EventLog::output(&address, &self.context.output);
        self.socket.send_to(&self.context.output, address).await
    }
}
