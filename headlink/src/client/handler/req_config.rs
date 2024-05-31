use std::collections::HashMap;
use std::net::Ipv4Addr;
use futures_util::StreamExt;
use ip_network::{IpNetwork, Ipv4Network};
use sea_orm::*;
use sea_orm::ActiveValue::Set;
use crate::client::dispatcher::{ClientRequest, RequestContext};
use crate::client::handler::{ExecuteResult, ToServerDataHandler};
use vlink_core::proto::pb::abi::{BcPeerEnter, ExtraTransport, PeerExtraTransport, ReqConfig, RespConfig};
use vlink_core::proto::pb::abi::to_client::ToClientData;
use crate::client::error::ExecuteError;
use crate::db::entity::prelude::{PeerActiveModel, PeerColumn, PeerEntity, PeerExtraTransportColumn, PeerExtraTransportEntity, PeerModel};
use crate::server::Peers;

impl ToServerDataHandler for ReqConfig {
    /// 发送配置
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult {
        //ctx.pubkey;, 查找peer
        let network = ctx.server.get_network(ctx.client_id.network_id).await?;
        let self_peer = network.peers
            .read_lock().await
            .get(ctx.client_id.pub_key.as_str()).cloned()
            .ok_or(ExecuteError::PeerNotFound)?;
        let mut peers = vec![];
        for (k, p) in network.peers.read_lock().await.iter() {
            if let Some(ip) = p.model.ip.as_ref() {
                //额外的连接信息
                peers.push(BcPeerEnter {
                    pub_key: k.to_string(),
                    ip: ip.to_string(),
                    endpoint_addr: p.online_info.as_ref().map(|e| e.endpoint_addr.clone()).unwrap_or(None),
                    port: p.online_info.as_ref().map(|e| e.port).unwrap_or(0),
                    last_con_type: None,
                    mode: 3,
                    is_online: p.online_info.is_some(),
                })
            }
        }

        // 获取ip
        let addr = match self_peer.model.ip.clone() {
            None => {
                //生成ip
                let gen_ip = generate_ip(network.cidr, &network.peers).await?;
                //更新ip
                let mut model = self_peer.model.clone();
                model.ip = Some(gen_ip.to_string());
                PeerEntity::update(PeerActiveModel {
                    ip: Set(Some(gen_ip.to_string())),
                    pub_key: Set(self_peer.model.pub_key.clone()),
                    ..Default::default()
                }).exec(ctx.conn())
                    .await?;

                network.peers.refresh_model(model).await;
                gen_ip
            }
            Some(e) => e.as_str().parse()?
        };
        //查询额外的传输层协议
        let transports = PeerExtraTransportEntity::find()
            .filter(PeerExtraTransportColumn::PeerId.eq(self_peer.model.id)
                .and(PeerExtraTransportColumn::Disabled.eq(false)))
            .all(ctx.conn())
            .await?;

        let extra_transports = transports.into_iter().map(|m| {
            ExtraTransport {
                proto: m.proto,
                params: m.params,
            }
        }).collect();

        let mut extra_endpoints_map = HashMap::new();
        for (k, v) in network.peers.read_lock().await.iter() {
            if let Some(e) = v.online_info.clone() {
                for (proto, end) in e.extra_endpoints.read_lock().await.iter() {
                    extra_endpoints_map.insert(k.clone(), (proto.to_string(), end.to_string()));
                }
            }
        };
        let mut peer_extra_transports = vec![];
        for (k, v) in extra_endpoints_map {
            //todo 校验协议是否对该peer 可用
            peer_extra_transports.push(PeerExtraTransport {
                target_pub_key: k.to_string(),
                proto: v.0,
                endpoint: v.1,
                index: 0,
            });
        }

        let resp = RespConfig {
            network_id: network.network_id,
            address: addr.into(),
            mask: network.cidr.netmask() as u32,
            network: network.cidr.network_address().into(),
            port: self_peer.model.port.unwrap_or(0) as u32,
            ipv6_addr: None,
            peers,
            extra_transports,
            peer_extra_transports,
        };
        ctx.send_resp(ToClientData::RespConfig(resp)).await?;
        Ok(())
    }
}

pub async fn generate_ip(network: Ipv4Network, peers: &Peers) -> anyhow::Result<Ipv4Addr> {
    let peer_ips: Vec<String> = peers.read_lock().await.iter().filter(|p| p.1.model.ip.is_some())
        .map(|p| p.1.model.ip.clone().unwrap())
        .collect();

    Ok(network.into_iter()
        .skip(2)
        .find(|i| {
            peer_ips.iter().all(|p| p.as_str() != i.to_string().as_str())
        }).ok_or(anyhow::anyhow!("ip地址不够"))?)
}


#[cfg(test)]
pub mod test {
    use std::net::Ipv4Addr;
    use ip_network::IpNetwork;

    #[test]
    pub fn test() {
        //let start=
        let subnet: Ipv4Addr = "192.168.68.0".parse().unwrap();
        // let subnet_mask=
        let net = ip_network::IpNetwork::new(subnet, 24).unwrap();
        let sub_mask = vlink_core::utils::network::bite_mask(24);
        // net.contains();

        match net {
            IpNetwork::V4(i) => {
                i.into_iter()
                    .skip(1)
                    .for_each(|i| {
                        println!("{}", i);
                    });
            }
            IpNetwork::V6(_) => {
                todo!("为实现ipv6");
            }
        }
    }
}