use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use crate::{LocalStaticSecret};
use crate::device::peer::cidr::Cidr;

#[derive(Default, Clone, Debug, )]
pub struct PeerConfig {
    pub public_key: [u8; 32],
    ///节点接受的ip
    pub allowed_ips: HashSet<Cidr>,
    /// 连接地址
    pub endpoint: Option<SocketAddr>,
    pub preshared_key: Option<[u8; 32]>,
    /// 延迟连接
    pub lazy: bool,
    /// 不加密
    /// 不加密无法与标准wireguard互通
    pub no_encrypt: bool,
    /// 连接时间
    pub persistent_keepalive: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub private_key: [u8; 32],
    pub fwmark: u32,
    pub port: u16,
    pub peers: HashMap<[u8; 32], PeerConfig>,
    pub address: Ipv4Addr,
    //网络
    pub network: Ipv4Addr,
    pub netmask: u8,

}

impl DeviceConfig {
    #[inline(always)]
    pub fn private_key(mut self, key: [u8; 32]) -> Self {
        self.private_key = key;
        self
    }

    // #[inline(always)]
    // pub fn listen_addr_v4(mut self, addr: Ipv4Addr) -> Self {
    //     self.listen_addrs.0 = addr;
    //     self
    // }
    //
    // #[inline(always)]
    // pub fn listen_addr_v6(mut self, addr: Ipv6Addr) -> Self {
    //     self.listen_addrs.1 = addr;
    //     self
    // }
    //
    // #[inline(always)]
    // pub fn listen_port(mut self, port: u16) -> Self {
    //     self.listen_port = port;
    //     self
    // }

    #[inline(always)]
    pub fn peer(mut self, peer: PeerConfig) -> Self {
        self.peers.insert(peer.public_key, peer);
        self
    }

    #[inline(always)]
    pub fn local_secret(&self) -> LocalStaticSecret {
        LocalStaticSecret::new(self.private_key)
    }
}

