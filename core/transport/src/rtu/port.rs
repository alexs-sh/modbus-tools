extern crate codec;
extern crate frame;

use crate::{settings::Settings, Handler, Request, Response};
use codec::rtu::RtuCodec;
use frame::{RequestFrame, ResponseFrame};
use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use std::io::{Error, ErrorKind};
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio_serial::{Parity, SerialPort, SerialPortBuilderExt, SerialStream, StopBits};
use tokio_util::codec::Framed;
use uuid::{self, Uuid};

struct PortSettings {
    name: String,
    speed: u32,
    parity: Parity,
    stop_bits: StopBits,
}

impl FromStr for PortSettings {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name: String = s.chars().take_while(|c| *c != ':').collect(); //&s[..delim_pos];
        let params: String = s.chars().skip_while(|c| *c != ':').skip(1).collect();
        let info: Vec<&str> = params.split('-').collect();

        if name.len() < 4 {
            return Err("name is too short");
        }

        if info.len() < 4 {
            return Err("not enough port parameters");
        }

        let speed = u32::from_str(info[0]).map_err(|_| "invalid speed")?;
        let parity = match info[2] {
            "N" => Ok(Parity::None),
            "E" => Ok(Parity::Even),
            "O" => Ok(Parity::Odd),
            _ => Err("invalid parity"),
        }?;

        let stop_bits = match info[3] {
            "1" => Ok(StopBits::One),
            "2" => Ok(StopBits::Two),
            _ => Err("invalid stop bits"),
        }?;

        Ok(PortSettings {
            name,
            speed,
            parity,
            stop_bits,
        })
    }
}

pub struct RtuPort {
    io: Framed<SerialStream, RtuCodec>,
    request_tx: mpsc::Sender<Request>,
    response_tx: mpsc::Sender<Response>,
    response_rx: mpsc::Receiver<Response>,
}

impl RtuPort {
    pub async fn build(settings: Settings) -> Result<Handler, Error> {
        let address = settings.address.get();
        let parameters = PortSettings::from_str(address).map_err(|err| {
            error!("{}", err);
            Error::new(ErrorKind::Other, "invalid port settings")
        })?;

        let port = tokio_serial::new(parameters.name, parameters.speed)
            .parity(parameters.parity)
            .stop_bits(parameters.stop_bits)
            .open_native_async()?;

        port.clear(tokio_serial::ClearBuffer::All)?;

        let codec = RtuCodec::new();
        let io = Framed::new(port, codec);

        let (tx, rx) = mpsc::channel(settings.nmsg);
        let (response_tx, response_rx) = mpsc::channel(1);
        let server = RtuPort {
            io,
            request_tx: tx,
            response_tx,
            response_rx,
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
            payload: request,
            response_tx: Some(self.response_tx.clone()),
        };
        debug!("recv request from serial: {:?}", request.payload);
        let _ = self.request_tx.send(request).await;
    }

    async fn send_response(&mut self, response: Response) {
        debug!("send response to serial: {:?}", response.payload);
        let response = ResponseFrame::from_parts(0, response.payload.slave, response.payload.pdu);
        let _ = self.io.send(response).await;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_settings() {
        assert_eq!(PortSettings::from_str(":").is_err(), true);
        assert_eq!(PortSettings::from_str("").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0:").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0:9600").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0:9600-8").is_err(), true);
        assert_eq!(
            PortSettings::from_str("/dev/ttyUSB0:9600-8-N").is_err(),
            true
        );
        let correct = PortSettings::from_str("/dev/ttyUSB0:9600-8-N-1").unwrap();
        assert_eq!(correct.name, "/dev/ttyUSB0");
        assert_eq!(correct.speed, 9600);
        assert_eq!(correct.parity, Parity::None);
        assert_eq!(correct.stop_bits, StopBits::One);
    }
}
