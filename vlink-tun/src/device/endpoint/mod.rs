use std::fmt::{Debug, Formatter};
use std::io;
use std::net::SocketAddr;
use crate::device::transport::{Transport, TransportDispatcher, TransportInbound, TransportOutbound, TransportWrapper};


/// peer->endpoints->peer
/// 点对点传输端点
#[derive(Clone)]
#[deprecated]
pub struct Endpoint {
    // transport_outbound: Box<dyn TransportInbound>,
    // transport_outbound: Box<dyn TransportOutbound>,
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
 /*   pub fn new(transport:  Box<dyn TransportOutbound>,dst: SocketAddr) -> Self {
        Self { transport_outbound: transport, dst }
    }*/


    #[inline]
    pub async fn send(&self, buf: &[u8]) -> Result<(), io::Error> {
        // self.transport_outbound.send_to(buf, self.dst).await?;
        Ok(())
    }

    /// Returns the destination of the endpoint.
    #[inline(always)]
    pub fn dst(&self) -> SocketAddr {
        self.dst
    }
}