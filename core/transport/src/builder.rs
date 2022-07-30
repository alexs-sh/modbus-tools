use crate::{
    settings::{Settings, TransportAddress},
    tcp::server::TcpServer,
    udp::server::UdpServer,
    Request,
};

use futures::Stream;
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
        _ => unimplemented!(),
    }
}
