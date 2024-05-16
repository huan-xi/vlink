use std::net::Ipv4Addr;
use crate::client::ClientConnect;

#[derive(Clone)]
pub struct VlinkPeer {
    pub(crate) connect: ClientConnect,
    pub(crate) addr: Ipv4Addr,
    pub(crate) port: u32,
}