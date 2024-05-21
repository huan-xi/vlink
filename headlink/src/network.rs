use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use futures_util::future::join_all;
use futures_util::SinkExt;
use ip_network::{IpNetwork, Ipv4Network};
use vlink_core::proto::pb::abi::to_client::ToClientData;
use vlink_core::proto::pb::abi::ToClient;
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

pub struct VlinkNetworkInner {
    pub network_id: i64,
    pub cidr: Ipv4Network,
    // pub online_peers: HashSet<String>,
    pub peers: Peers,
}

impl VlinkNetworkInner {
    pub async fn broadcast(&self, data: ToClientData, exclude: Vec<String>) {
        let mut task = vec![];

        for (k,peer) in self.peers.read_lock().await.iter() {
            if exclude.contains(k) {
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
}