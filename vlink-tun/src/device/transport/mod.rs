use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Error;
use std::net::SocketAddr;
use async_trait::async_trait;
use tokio::select;
use crate::device::endpoint::Endpoint;
use crate::device::transport::udp::UdpTransport;

/// 标准的udp 协议
pub mod udp;


/// 传输层入口数据
#[async_trait]
#[deprecated]
pub trait TransportInbound: Sync + Send + Unpin + Display + 'static + Debug {
    /// Receives data from the endpoint.
    async fn recv_from(&mut self) -> Result<(Endpoint, Vec<u8>), io::Error>;
}

/// 传输层出口
#[async_trait]
pub trait TransportOutbound: Sync + Send + Unpin + Display + 'static + Debug + Clone + Sized {
    /// Sends data to the given endpoint.
    async fn send_to(&self, data: &[u8], dst: SocketAddr) -> Result<(), io::Error>;
}

#[async_trait]
pub trait Transport: TransportInbound + TransportOutbound {}


#[derive(Debug, Clone)]
pub enum TransportWrapper {
    Udp(UdpTransport)
}


#[derive(Debug, Clone)]
pub struct TransportDispatcher {
    pub(crate) transports: Vec<TransportWrapper>,
}

#[async_trait]
impl TransportInbound for TransportDispatcher {
    async fn recv_from(&mut self) -> Result<(Endpoint, Vec<u8>), Error> {
        todo!();
    }
}

impl TransportDispatcher {
    pub fn new(transports: Vec<TransportWrapper>) -> Self {
        Self {
            transports
        }
    }
}


impl Display for TransportDispatcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for TransportWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

