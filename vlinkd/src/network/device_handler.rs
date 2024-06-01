use std::str::FromStr;
use log::{info, warn};
use vlink_core::base64::encode_base64;
use vlink_core::proto::pb::abi::{DevHandshakeComplete, ExtraEndpoint};
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_tun::device::event::DeviceEvent;
use crate::network::{ExtraProto, ExtraProtoStatus, VlinkNetworkManagerInner};

/// 通过设备产生的事件，去处理网络
pub async fn handle_device_event(net: VlinkNetworkManagerInner, event: DeviceEvent) -> anyhow::Result<()> {
    let cc = net.client.clone();
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
        }
        //协议失败
        _ => {}
    }
    Ok(())
}