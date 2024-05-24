use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use ip_network::IpNetwork;
use serde::{Deserialize, Serialize};
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

impl PeerConfig {}


#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub private_key: [u8; 32],
    pub fwmark: u32,
    pub port: u16,
    pub peers: HashMap<[u8; 32], PeerConfig>,
    pub address: Ipv4Addr,
    //网络
    pub network: IpNetwork,
}

#[derive(Debug, Clone)]
pub struct ArgConfig {
    pub endpoint_addr: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransportType {
    NatUdp,
}

/// 传输层配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportConfig {
    pub trans_type: TransportType,
    pub params: String,
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


