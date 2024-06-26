use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use futures_util::future::join_all;
use futures_util::SinkExt;
use ip_network::{IpNetwork, Ipv4Network};
use sea_orm::ColIdx;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use vlink_core::proto::pb::abi::{BcPeerLevel, ToClient};
use vlink_core::rw_map::RwMap;
use crate::peer::VlinkPeer;
use crate::server::{Peers, VlinkServer};

#[derive(Clone)]
pub struct VlinkNetwork {
    pub inner: Arc<VlinkNetworkInner>,
}

impl Deref for VlinkNetwork {
    type Target = VlinkNetworkInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// 连接
#[derive(Clone)]
pub struct PeerConnect {
    /// true 是正向
    pub(crate) direction: bool,
    pub(crate) proto: String,

}


pub struct VlinkNetworkInner {
    pub network_id: i64,
    pub cidr: Ipv4Network,
    // pub online_peers: HashSet<String>,
    pub peers: Peers,
    pub connects: RwMap<String, PeerConnect>,
}

impl VlinkNetworkInner {

    ///下线设备
    pub async fn offline(&self, pub_key: &str) {
        self.peers.offline(pub_key).await;
        self.broadcast(ToClientData::PeerLeave(BcPeerLevel {
            pub_key: pub_key.to_string(),
        }), pub_key).await;
    }


    pub async fn broadcast_by<F>(&self, data: ToClientData, predict: F)
        where F: Fn(&String, &VlinkPeer) -> bool {
        let mut task = vec![];
        for (k, peer) in self.peers.read_lock().await.iter() {
            if !predict(k, peer) {
                continue;
            }
            if let Some(s) = &peer.online_info {
                let conn = s.connect.clone();
                let data = data.clone();
                task.push(async move {
                    let _ = conn.send(None, data).await;
                })
            }
        }
        join_all(task).await;
    }

    pub async fn broadcast_to(&self, data: ToClientData, include: Vec<String>) {
        if include.is_empty() {
            return;
        }
        self.broadcast_by(data, |k, _| include.contains(&k)).await;
    }
    pub async fn broadcast(&self, data: ToClientData, exclude: &str) {
        self.broadcast_by(data, |k, _| !exclude.eq(k.as_str())).await;
    }
    pub async fn broadcast_excludes(&self, data: ToClientData, exclude: Vec<String>) {
        self.broadcast_by(data, |k, _| !exclude.contains(&k)).await;
    }
}