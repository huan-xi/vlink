use std::collections::{HashMap, HashSet};
use vlink_core::proto::pb::abi::{ConnectionMode, PeerEnter};
use vlink_core::proto::pb::abi::BcPeerEnter;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use vlink_core::rw_map::RwMap;
use crate::client::dispatcher::{ClientRequest, RequestContext};
use crate::client::error::ExecuteError;
use crate::client::handler::{ExecuteResult, ToServerDataHandler};
use crate::peer::OnlineInfo;

/// 处理客户端首次连接或者恢复连接
impl ToServerDataHandler for PeerEnter {
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult {
        let pub_key = ctx.client_id.pub_key.clone();
        let network = ctx.server.get_network(ctx.client_id.network_id).await?;
        let mut lock = network.peers.write_lock().await;
        let mut peer = lock.get_mut(ctx.client_id.pub_key.as_str())
            .ok_or(ExecuteError::PeerNotFound)?;
        let mut extra_endpoints = vec![];
        for x in self.extra_endpoints.iter() {
            extra_endpoints.push((x.proto.clone(), x.endpoint.clone()));
        }

        let online_info = OnlineInfo {
            connect: ctx.client.clone(),
            port: self.port,
            endpoint_addr: self.endpoint_addr.clone(),
            extra_endpoints: RwMap::from(extra_endpoints),
        };
        if self.ip.as_str() != peer.model.ip.clone().unwrap_or("".to_string()).as_str() {
            return Err(ExecuteError::IpNotMatch);
        };
        peer.online_info = Some(online_info);
        //客户端进入->enter
        drop(lock);
        network.broadcast(ToClientData::PeerEnter(BcPeerEnter {
            pub_key: pub_key.clone(),
            ip: self.ip.clone(),
            endpoint_addr: self.endpoint_addr.clone(),
            port: self.port,
            last_con_type: None,
            mode: i32::from(ConnectionMode::Bidirectional),
            is_online: true,
        }), pub_key.as_str()).await;
        Ok(())
    }
}