use std::collections::HashSet;
use std::net::SocketAddr;
use vlink_core::proto::pb::abi::BcPeerEnter;
use vlink_tun::device::peer::cidr::Cidr;
use vlink_tun::PeerConfig;

pub fn bc_peer_enter2peer_config(p: &BcPeerEnter) -> anyhow::Result<PeerConfig> {
    let pk = vlink_core::base64::decode_base64(p.pub_key.as_str())?;
    let mut allowed_ips = HashSet::new();
    allowed_ips.insert(Cidr::new(p.ip.parse().unwrap(), 32));
    Ok(PeerConfig {
        public_key: pk.try_into().unwrap(),
        allowed_ips,
        endpoint: match p.endpoint_addr.clone() {
            None => { None }
            Some(addr) => {
                Some(SocketAddr::new(addr.parse()?, p.port as u16))
            }
        },
        preshared_key: None,
        lazy: false,
        is_online: p.is_online,
        no_encrypt: false,
        persistent_keepalive: None,
        ip_addr: p.ip.clone(),
    })
}

