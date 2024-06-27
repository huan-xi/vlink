use std::sync::Arc;
use std::time::Duration;
use log::{debug, error, info};
use tokio::sync::mpsc;
use tokio::time;
use vlink_core::proto::pb::abi::PeerExtraTransport;
use vlink_tun::device::peer::Peer;
use vlink_tun::InboundResult;
use crate::transport::proto::nat_tcp::{NatTcpTransportClient, NatTcpTransportParam};
use crate::transport::proto::nat_udp::NatUdpTransportClient;
use crate::transport::proto::relay_transport::RelayTransport;

/// 协议选择器间隔
const SELECTOR_INTERVAL: u64 = 10;

/// 扩展传输层选择器
/// 选择扩展协议，更新peer的endpoint


pub struct ExtTransportSelector {
    peer: Arc<Peer>,
    transports: Vec<PeerExtraTransport>,
}

/// 对目标节点选择扩展协议
impl ExtTransportSelector {
    pub fn new(peer: Arc<Peer>, inbound_tx: mpsc::Sender<InboundResult>,
               transports: Vec<PeerExtraTransport>, relay: Arc<RelayTransport>) -> Self {
        let peer_c = Arc::clone(&peer);
        let transports_c = transports.clone();
        let inbound_tx_c = inbound_tx.clone();
        let relay_c = relay.clone();

        tokio::spawn(async move {
            // 启动循环检测peer 的 endpoint
            let mut interval = time::interval(Duration::from_secs(SELECTOR_INTERVAL));
            loop {
                let inbound_tx_c = inbound_tx_c.clone();
                peer_c.await_online().await;
                if peer_c.endpoint.read().unwrap().is_none() {
                    //启动对应的传输层连接器
                    if transports_c.len() > 0 {
                        if let Some(e) = transports_c.get(0) {
                            let proto = e.proto.clone();
                            match proto.as_str() {
                                "NatUdp" => {
                                    let c = NatUdpTransportClient::new(peer_c.clone(), inbound_tx_c, e.endpoint.clone()).await.unwrap();
                                    let e = c.endpoint();
                                    info!("start endpoint success {e}");
                                    peer_c.update_endpoint(e);
                                }
                                "NatTcp" => {
                                    match NatTcpTransportClient::spawn(peer_c.clone(), inbound_tx_c, e.endpoint.clone()).await {
                                        Ok(c) => {
                                            let e = c.endpoint();
                                            info!("start endpoint success {e}");
                                            peer_c.update_endpoint(e);
                                        }
                                        Err(e) => {
                                            error!("start endpoint failed {e}");
                                        }
                                    }
                                }
                                _ => {
                                    error!("not support proto {proto}");
                                }
                            }
                        }
                    } else {
                        //启动中继
                        debug!("require_reply for {:?}", peer_c);
                        let _ = relay_c.require_reply(peer_c.pub_key.as_bytes()).await;
                    }
                    //NatUdpTransportClient
                }

                interval.tick().await;
            }
            //peer_c.update_endpoint().await;
        });

        Self {
            peer,
            transports,
        }
    }

    pub fn insert(&mut self, ps: Vec<PeerExtraTransport>) {}
}