use std::sync::Arc;
use tokio::sync::RwLock;
use vlink_core::proto::pb::abi::{ExtraEndpoint, PeerEnter};
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_core::rw_map::RwMap;
use vlink_tun::Device;
use vlink_tun::device::config::ArgConfig;
use crate::client::VlinkClient;
use crate::network::{ExtraProto, ExtraProtoStatus};

/// 处理客户端连接成功
/// 上报端点
///
pub async fn handler_connected(client: Arc<VlinkClient>, device: Arc<RwLock<Option<Device>>>, args: &ArgConfig, es: RwMap<ExtraProto, ExtraProtoStatus>) -> anyhow::Result<()> {
    let (ip, port) = {
        let get_info = |dev: &Device| {
            (dev.tun_addr.clone().to_string(), dev.port as u32)
        };
        get_info(device.read().await.as_ref().ok_or(anyhow::anyhow!("device is none"))?)
    };
    //已启动的协议和端口

    let mut extra_endpoints = vec![];
    for (proto, v) in es.read_lock().await.iter() {

        if let Some(endpoint) = v.endpoint.clone() {
            extra_endpoints.push(ExtraEndpoint {
                proto: proto.as_ref().to_string(),
                endpoint,
            })
        };
    }

    client.send(ToServerData::PeerEnter(PeerEnter {
        ip,
        endpoint_addr: args.endpoint_addr.clone(),
        port,
        extra_endpoints,
    })).await?;
    Ok(())
}
////// 进入网络信息
// /// 本机ip+udp port
// pub async fn send_enter(client: &VlinkClient, device: &Device, arg: &ArgConfig) -> anyhow::Result<()> {
//     client.send(ToServerData::PeerEnter(PeerEnter {
//         ip: device.tun_addr.to_string(),
//         endpoint_addr: arg.endpoint_addr.clone(),
//         port: device.port as u32,
//     })).await?;
//     Ok(())
// }