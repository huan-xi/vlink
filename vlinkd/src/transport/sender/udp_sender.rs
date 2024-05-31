use std::fmt::{Debug, Display, Formatter};
use std::io::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::net::UdpSocket;
use vlink_tun::{BoxCloneOutboundSender, OutboundSender};
use crate::transport::forward::udp::UdpForwarder;
use crate::transport::proto::nat_udp::PROTO_NAME;

#[derive(Clone)]
pub struct Ipv4UdpOutboundSender {
    pub(crate) dst: SocketAddr,
    pub(crate) socket: Arc<UdpSocket>,
}

impl BoxCloneOutboundSender for Ipv4UdpOutboundSender {
    fn box_clone(&self) -> Box<dyn OutboundSender> {
        Box::new(self.clone())
    }
}

impl Debug for Ipv4UdpOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Ipv4UdpOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ipv4Udp -> ({})", self.dst)
    }
}

#[async_trait]
impl OutboundSender for Ipv4UdpOutboundSender {
    async fn send(&self, data: &[u8]) -> Result<(), Error> {
        self.socket.send_to(data, self.dst).await?;
        Ok(())
    }

    fn dst(&self) -> SocketAddr {
        self.dst
    }

    fn protocol(&self) -> String {
        PROTO_NAME.to_string()
    }
}

