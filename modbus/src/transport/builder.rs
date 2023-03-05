use crate::transport::{
    rtu::slave::RtuSlaveChannel,
    settings::{Settings, TransportAddress},
    tcp::server::TcpServer,
    udp::server::UdpServer,
    Request,
};

use futures::{Stream, StreamExt};
use log::info;
use std::io::Error;

pub async fn build(settings: Settings) -> Result<impl Stream<Item = Request>, Error> {
    match &settings.address {
        TransportAddress::Tcp(address) => {
            info!("start tcp server {}", address);
            let handler = TcpServer::build(settings).await?;
            Ok(handler.to_stream())
        }
        TransportAddress::Udp(address) => {
            info!("start udp server {}", address);
            let handler = UdpServer::build(settings).await?;
            Ok(handler.to_stream())
        }
        TransportAddress::Serial(address) => {
            info!("start rtu slave {}", address);
            let handler = RtuSlaveChannel::build(settings).await?;
            Ok(handler.to_stream())
        }
    }
}

pub struct SlaveTransport {}

//TODO:sas: For now, Fn handler is good enough. But it's a nice place for using Service
pub async fn build_slave<H>(settings: Settings, handler: H) -> Result<SlaveTransport, Error>
where
    H: Fn(Request) + std::marker::Send + 'static,
{
    let mut stream = build(settings).await?;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                    Some(request) = stream.next() => {
                        handler(request);
                }
            }
        }
    });

    Ok(SlaveTransport {})
}
