use std::str::FromStr;
use std::sync::Arc;
use log::{info, warn};
use vlink_core::base64::encode_base64;
use vlink_core::proto::pb::abi::{DevHandshakeComplete, ExtraEndpoint};
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_tun::Device;
use vlink_tun::device::event::DeviceEvent;
use crate::network::{ExtraProto, ExtraProtoStatus, VlinkNetworkManager, VlinkNetworkManagerInner};

/// 通过设备产生的事件，去处理网络
pub async fn handle_device_event(net: VlinkNetworkManager, dev: Arc<Device>, event: DeviceEvent) -> anyhow::Result<()> {
    let cc = net.client.clone();
    let peers =dev.peers.clone();
    //处理设备事件
    match event {
        DeviceEvent::HandshakeComplete(data) => {
            // 发送握手完成
            let _ = cc.send(ToServerData::DevHandshakeComplete(DevHandshakeComplete {
                target_pub_key: encode_base64(data.pub_key.as_bytes()),
                proto: data.proto,
            })).await;
        }
        DeviceEvent::ExtraEndpointSuccess(data) => {
            info!("extra endpoint success:{:?}", data);
            let proto = ExtraProto::from_str(data.proto.as_str())?;
            net.extra_status.write_lock().await.insert(proto, ExtraProtoStatus {
                endpoint: Some(data.endpoint.clone()),
                running: true,
                error: None,
            });
            //发送更新端点
            let _ = net.client.send(ToServerData::UpdateExtraEndpoint(ExtraEndpoint {
                proto: data.proto,
                endpoint: data.endpoint,
            })).await;
        }
        DeviceEvent::SessionFailed(peer) => {
            warn!("session failed:{:?}", peer);
        }
        DeviceEvent::NoEndpoint((pub_key, ip_addr)) => {
            warn!("no endpoint for peer:{}", ip_addr.as_str());
            //启动中继协议

            //net.relay_transport.read().start_end
        }
        DeviceEvent::TransportFailed(data) => {
            warn!("transport failed:{:?}", data);
        }
        DeviceEvent::PeerEndpointFailed(p) => {
            if let Some(peer) = peers.read().unwrap().get_by_key(p.as_bytes()) {
                peer.clear_endpoint();
            }
        }
        //协议失败
        _ => {}
    }
    Ok(())
}