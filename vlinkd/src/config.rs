use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::net::SocketAddr;
use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use vlink_core::proto::pb::abi::{BcPeerEnter, PeerExtraTransport};
use vlink_core::secret::VlinkStaticSecret;
use vlink_tun::device::config::{ArgConfig, TransportConfig};
use vlink_tun::{DeviceConfig, PeerConfig, PeerStaticSecret};
use vlink_tun::device::peer::cidr::Cidr;

#[derive(Deserialize, Serialize, Debug)]
pub struct StorageConfig {
    pub secret: VlinkStaticSecret,
}



#[derive(Debug, Clone)]
pub struct PeersConfig {}


#[derive(Debug, Clone)]
pub struct VlinkNetworkConfig {
    pub tun_name: Option<String>,
    pub device_config: DeviceConfig,
    pub arg_config: ArgConfig,

    pub transports: Vec<TransportConfig>,
    pub stun_servers: Vec<String>,

    // peer 扩展协议
    pub peer_extra_transports: Vec<PeerExtraTransport>,
    // pub test: RespConfig,
}


