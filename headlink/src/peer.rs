use std::net::{IpAddr, Ipv4Addr};
use crate::client::ClientConnect;

#[derive(Clone)]
pub struct VlinkPeer {
    pub(crate) connect: ClientConnect,
    pub(crate) endpoint_addr: Option<IpAddr>,
    pub addr: Ipv4Addr,
    pub(crate) port: u32,
}

