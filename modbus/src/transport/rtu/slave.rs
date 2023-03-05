use super::port::{self, PortSettings};
use crate::codec::slave::SlaveCodec;
use crate::frame::prelude::*;
use crate::transport::{event::EventLog, prelude::*};
use std::io::{Error, ErrorKind};
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_serial::SerialStream;
use uuid::{self, Uuid};

// TODO: Reset buffer if no reading for N ms. It better to make configurable
const INACTIVE_TIMEOUT: u64 = 250;

pub struct RtuSlaveChannel {
    stream: SerialStream,
    context: IoContext,
    request_tx: mpsc::UnboundedSender<Request>,
    response_tx: mpsc::UnboundedSender<Response>,
    response_rx: mpsc::UnboundedReceiver<Response>,

    name: String,
}

impl RtuSlaveChannel {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let address = settings.address.get();
        let parameters = PortSettings::from_str(address)
            .map_err(|_| Error::new(ErrorKind::Other, "invalid port settings"))?;

        let port = port::build(parameters)?;
        let codec = SlaveCodec::new_rtu();
        let context = IoContext::new(codec);
        let (tx, rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        let server = RtuSlaveChannel {
            stream: port,
            context,
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
        tokio::spawn(async move {
            loop {
                let _ = self.run().await.map_err(|err| {
                    self.context.reset();
                    EventLog::error(&self.name, &err);
                });
            }
        });
    }

    fn reset(&mut self, reason: &str) {
        if !self.context.input.is_empty() {
            EventLog::warning(&self.name, &reason);
        }
        self.context.reset();
    }

    async fn run(&mut self) -> Result<(), Error> {
        // read request with timeout
        let read = tokio::time::timeout(
            std::time::Duration::from_millis(INACTIVE_TIMEOUT),
            self.stream.read_buf(&mut self.context.input),
        );

        tokio::select! {
            input = read => {
                match input {
                    //read:timeout
                    Err(_) => {
                        self.reset("reset by timeout");
                        Ok(())
                    },

                    //read next chunk
                    Ok(Ok(_nbytes)) => {
                        self.on_input().await
                    },
                    //read failed
                    Ok(Err(e)) => {
                        Err(e)
                    },
                }
            },
            // got response
            response = self.response_rx.recv() => {
                self.on_response(response).await
            }
        }
    }

    async fn on_input(&mut self) -> Result<(), Error> {
        EventLog::input(&self.name, &self.context.input);
        let Some(request) = self.context.decode()? else { return Ok(()) };
        self.on_request(request).await;
        Ok(())
    }

    async fn on_request(&mut self, frame: RequestFrame) {
        let uuid = Uuid::new_v4();
        let request = Request {
            uuid,
            slave: frame.slave,
            pdu: frame.pdu,
            response_tx: Some(self.response_tx.clone()),
        };

        EventLog::request(&self.name, &request);
        let _ = self.request_tx.send(request);
    }

    async fn on_response(&mut self, response: Option<Response>) -> Result<(), Error> {
        if let Some(response) = response {
            EventLog::response(&self.name, &response);
            self.context
                .encode(ResponseFrame::from_parts(0, response.slave, response.pdu))?;
            self.on_output().await?;
        }
        Ok(())
    }

    async fn on_output(&mut self) -> Result<(), Error> {
        EventLog::output(&self.name, &self.context.output);
        self.stream.write_all(&self.context.output).await
    }
}
