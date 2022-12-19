extern crate codec;
extern crate frame;

use super::port::{self, PortSettings};
use crate::{settings::Settings, Handler, Request, Response};
use codec::helpers;
use codec::rtu::RtuCodec;
use frame::{RequestFrame, ResponseFrame};
use futures::{SinkExt, StreamExt};
use log::{error, warn};
use std::io::{Error, ErrorKind};
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;
use uuid::{self, Uuid};

pub struct RtuSlaveChannel {
    io: Framed<SerialStream, RtuCodec>,
    request_tx: mpsc::Sender<Request>,
    response_tx: mpsc::Sender<Response>,
    response_rx: mpsc::Receiver<Response>,
    name: String,
}

impl RtuSlaveChannel {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let address = settings.address.get();
        let parameters = PortSettings::from_str(address).map_err(|err| {
            error!("{}", err);
            Error::new(ErrorKind::Other, "invalid port settings")
        })?;

        let port = port::build(parameters)?;

        let codec = RtuCodec::new(address);
        let io = Framed::new(port, codec);

        let (tx, rx) = mpsc::channel(settings.nmsg);
        let (response_tx, response_rx) = mpsc::channel(1);
        let server = RtuSlaveChannel {
            io,
            request_tx: tx,
            response_tx,
            response_rx,
            name: address.to_owned(),
        };

        let handler = Handler { request_rx: rx };
        server.spawn();
        Ok(handler)
    }

    pub fn spawn(mut self) {
        tokio::spawn(async move { while self.run().await {} });
    }

    fn reset(&mut self) {
        self.io.codec_mut().reset();
        self.io.read_buffer_mut().clear();
    }

    async fn run(&mut self) -> bool {
        let read_op = tokio::time::timeout(std::time::Duration::from_millis(1000), self.io.next());
        tokio::select! {
            read = read_op => {
                match read {
                    //timeout
                    Err(_) => {
                        if self.io.codec().in_progress() {
                            warn!("reset buffer by timeout");
                            self.reset();
                        }
                    },

                    //read done
                    Ok(Some(Ok(request))) => {
                        self.start_request(request).await;
                    },
                    Ok(Some(Err(err))) => {
                        error!("serial error:{:?}",err);
                        self.reset();
                    },
                    Ok(None) => {
                        self.reset()
                    },
                }
            }

            response = self.response_rx.recv() => {
                if let Some(response) = response {
                     self.send_response(response).await;
                }
            }
        };
        true
    }

    async fn start_request(&mut self, request: RequestFrame) {
        let uuid = Uuid::new_v4();
        let request = Request {
            uuid,
            slave: request.slave,
            pdu: request.pdu,
            response_tx: Some(self.response_tx.clone()),
        };

        helpers::log_message(&self.name, &request);
        let _ = self.request_tx.send(request).await;
    }

    async fn send_response(&mut self, response: Response) {
        helpers::log_message(&self.name, &response);
        let frame = ResponseFrame::from_parts(0, response.slave, response.pdu);
        let _ = self.io.send(frame).await;
    }
}
