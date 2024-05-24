use std::net::SocketAddr;
use crate::device::endpoint::Endpoint;
use crate::device::transport::{TransportDispatcher};


pub(super) struct Inbound {
    // ///传输层列表,udp,tcp,nat_udp,nat_tcp,derp,
    pub(crate) transports: Vec<TransportDispatcher>,
}

impl Inbound {
    #[inline(always)]
    pub fn transport(&self) -> TransportDispatcher {
        self.transports[0].clone()
    }


    #[inline(always)]
    pub fn endpoint_for(&self, dst: SocketAddr) -> Endpoint {
        Endpoint::new(self.transport(), dst)
    }

}