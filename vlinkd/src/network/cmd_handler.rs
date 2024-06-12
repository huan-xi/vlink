use std::sync::Arc;
use log::warn;
use tokio::sync::RwLock;
use vlink_core::base64::decode_base64;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use vlink_tun::Device;

pub async fn handle_to_client_data(data: ToClientData, device: Arc<Device>)->anyhow::Result<()> {
    match data {
        ToClientData::PeerEnter(e) => {
            //标记节点在线
            let peer = device.get_peer_by_key(&decode_base64(e.pub_key.as_str())?.try_into().unwrap());
            match peer {
                None => {
                    warn!("peer not found");
                }
                Some(e) => {
                    e.set_online(true);
                }
            }
        }
        ToClientData::PeerLeave(e) => {
            //标记节点离线
            let peer = device.get_peer_by_key(&decode_base64(e.pub_key.as_str())?.try_into().unwrap());
            match peer {
                None => {
                    warn!("peer not found");
                }
                Some(e) => {
                    e.set_online(false);
                }
            }
        }
        _ => {}
    }
    Ok(())
}