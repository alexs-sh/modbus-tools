use crate::codec::slave::SlaveCodec;
use crate::frame::prelude::*;
use crate::transport::{event::EventLog, prelude::*};
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use uuid::{self, Uuid};

// TODO: Close client if no reading for N ms. It better to make configurable
const INACTIVE_TIMEOUT: u64 = 30000;

struct MsgInfo {
    uuid: Uuid,
    mbid: u16,
}

pub struct TcpServer {
    listener: TcpListener,
    request_tx: mpsc::UnboundedSender<Request>,
}

struct Client {
    stream: TcpStream,
    request_tx: mpsc::UnboundedSender<Request>,
    response_tx: mpsc::UnboundedSender<Response>,
    response_rx: mpsc::UnboundedReceiver<Response>,
    address: String,
    context: IoContext,
    wait_for: Option<MsgInfo>,
}

impl Client {
    fn spawn(mut self) {
        EventLog::info(&self.address, &"connected");
        tokio::spawn(async move { while self.run().await.is_ok() {} });
    }

    async fn run(&mut self) -> Result<(), Error> {
        let read = tokio::time::timeout(
            std::time::Duration::from_millis(INACTIVE_TIMEOUT),
            self.stream.read_buf(&mut self.context.input),
        );

        tokio::select! {
            result = read => {
                match result {
                    Err(e) => {
                        // timeout => close
                        EventLog::warning(&self.address, &"inactive timeout");
                        Err(Error::from(e))
                    }
                    Ok(Err(e)) => {
                        // read error => close
                        EventLog::error(&self.address, &e);
                        Err(e)
                    },

                    Ok(Ok(0)) => {
                        // close socket
                        Err(Error::new(ErrorKind::Other, "close"))
                    },
                    Ok(Ok(_nbytes)) =>
                    {
                        // got data. Try to process
                        self.on_input().await.map_err(|e|
                            {
                                EventLog::error(&self.address,&e);
                                e
                            })
                    },

                }
            },
            response = self.response_rx.recv() => {
                self.on_response(response).await
            }
        }
    }

    async fn on_input(&mut self) -> Result<(), Error> {
        EventLog::input(&self.address, &self.context.input);
        let Some(request) = self.context.decode()? else { return Ok(()) };
        self.on_request(request).await;
        Ok(())
    }

    async fn on_request(&mut self, frame: RequestFrame) {
        // make ids
        let uuid = Uuid::new_v4();
        let mbid = frame.id;

        // create request
        let request = Request {
            uuid,
            slave: frame.slave,
            pdu: frame.pdu,
            response_tx: Some(self.response_tx.clone()),
        };

        EventLog::request(&self.address, &request);

        // try to send to processor
        if self.request_tx.send(request).is_ok() {
            // save info about the request
            self.wait_for = Some(MsgInfo { uuid, mbid });
        } else {
            EventLog::warning(&self.address, &"can't process input request.TX overflow?");
        }
    }

    async fn on_response(&mut self, response: Option<Response>) -> Result<(), Error> {
        let Some(response) = response else { return Ok(()); };
        let resp_match = self
            .wait_for
            .as_ref()
            .map_or(false, |info| info.uuid == response.uuid);

        if resp_match {
            let info = self.wait_for.take().unwrap();
            let frame = ResponseFrame::from_parts(info.mbid, response.slave, response.pdu);
            self.on_output(frame).await?;
            self.context.reset();
        } else {
            EventLog::warning(&self.address, &"unknown response uuid");
        };
        Ok(())
    }

    async fn on_output(&mut self, frame: ResponseFrame) -> Result<(), Error> {
        self.context.encode(frame)?;
        EventLog::output(&self.address, &self.context.output);
        self.stream.write_all(&self.context.output[..]).await
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        EventLog::info(&self.address, &"close");
    }
}

impl TcpServer {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let listener = TcpListener::bind(settings.address.get()).await?;
        let (tx, rx) = mpsc::unbounded_channel();
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
        let (tx, rx) = mpsc::unbounded_channel();
        let address = address.to_string();
        let codec = SlaveCodec::new_tcp();
        let context = IoContext::new(codec);
        let client = Client {
            stream,
            request_tx: self.request_tx.clone(),
            response_tx: tx,
            response_rx: rx,
            address,
            context,
            wait_for: None,
        };
        client.spawn();
    }
}
