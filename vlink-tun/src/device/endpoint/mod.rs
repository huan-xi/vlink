use std::fmt::{Debug, Formatter};
use std::io;
use std::net::SocketAddr;
use crate::device::transport::{Transport, TransportDispatcher};


/// peer->endpoints->peer
/// 点对点传输端点
#[derive(Clone)]
pub struct Endpoint {
    transport: TransportDispatcher,
    dst: SocketAddr,
}

impl Debug for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("dst", &"hello")
            .finish()
        /* f.debug_struct("Endpoint")
             .field("dst", &self.dst.to_string())
             .finish()*/
    }
}

impl Endpoint {
    /// Creates a new endpoint with the given endpoint and destination.
    pub fn new(transport: TransportDispatcher, dst: SocketAddr) -> Self {
        Self { transport, dst }
    }


    #[inline]
    pub async fn send(&self, buf: &[u8]) -> Result<(), io::Error> {
        self.transport.send_to(buf, self.dst).await?;
        Ok(())
    }

    /// Returns the destination of the endpoint.
    #[inline(always)]
    pub fn dst(&self) -> SocketAddr {
        // self.dst
        todo!();
    }
}