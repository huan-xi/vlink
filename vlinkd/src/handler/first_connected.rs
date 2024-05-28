use std::net::IpAddr;
use std::sync::Arc;
use anyhow::anyhow;
use ip_network::IpNetwork;
use log::info;
use vlink_core::proto::pb::abi::ReqConfig;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_tun::device::config::ArgConfig;
use vlink_tun::DeviceConfig;
use crate::client::VlinkClient;
use crate::config::VlinkNetworkConfig;
use crate::handler::common::bc_peer_enter2peer_config;

pub async fn request_for_config(client: Arc<VlinkClient>, private_key: [u8; 32], args: &ArgConfig) -> anyhow::Result<VlinkNetworkConfig> {
    let resp = client.request(ToServerData::ReqConfig(ReqConfig {})).await?;
    info!("resp:{:?}",resp);
    let resp_config = if let ToClientData::RespConfig(config) = resp {
        config
    } else {
        return Err(anyhow!("响应数据错误错误"));
    };

    let network = IpNetwork::new(IpAddr::V4(resp_config.network.into()), resp_config.mask as u8)?;

    let mut device_config = DeviceConfig {
        private_key,
        fwmark: 0,
        port: {
            if resp_config.port > 0 {
                resp_config.port as u16
            } else {
                args.port.unwrap_or(crate::DEFAULT_PORT)
            }
        },
        peers: Default::default(),
        address: resp_config.address.into(),
        network,
    };
    for p in resp_config.peers.iter() {
        let c = bc_peer_enter2peer_config(p)?;
        device_config = device_config.peer(c);
    }

    let cfg = VlinkNetworkConfig {
        tun_name: None,
        device_config,
        arg_config: args.clone(),
        transports: vec![],
        stun_servers: vec![],
    };
    Ok(cfg)
}