use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Error;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use async_trait::async_trait;
use crate::device::endpoint::Endpoint;
use crate::device::transport::udp::UdpTransport;

pub mod udp;


#[derive(Debug, Clone)]
pub enum TransportDispatcher {
    Udp(UdpTransport)
}


impl Display for TransportDispatcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self { TransportDispatcher::Udp(udp) => { Debug::fmt(&udp, f) } }
    }
}

#[async_trait]
impl Transport for TransportDispatcher {
    fn ipv4(&self) -> Ipv4Addr {
        todo!()
    }

    fn ipv6(&self) -> Ipv6Addr {
        todo!()
    }

    fn port(&self) -> u16 {
        todo!()
    }

    async fn send_to(&self, data: &[u8], dst: SocketAddr) -> Result<(), Error> {
        match self {
            TransportDispatcher::Udp(u) => {
                u.send_to(data, dst).await
            }
        }
    }

    async fn recv_from(&mut self) -> Result<(Endpoint, Vec<u8>), Error> {
        match self { TransportDispatcher::Udp(udp) => { udp.recv_from().await } }

    }
}


#[async_trait]
pub trait Transport: Sync + Send + Unpin + Display + 'static + Debug {
    /// Binds to the given port and returns a new endpoint.
    /// When the port is 0, the implementation should choose a random port.
    // async fn bind(ipv4: Ipv4Addr, ipv6: Ipv6Addr, port: u16) -> Result<Self, io::Error>;

    fn ipv4(&self) -> Ipv4Addr;

    fn ipv6(&self) -> Ipv6Addr;

    /// Returns the port that the endpoint is bound to.
    fn port(&self) -> u16;

    // /// Sends data to the given endpoint.
    async fn send_to(&self, data: &[u8], dst: SocketAddr) -> Result<(), io::Error>;
    //
    // /// Receives data from the endpoint.
    async fn recv_from(&mut self) -> Result<(Endpoint, Vec<u8>), io::Error>;
}
