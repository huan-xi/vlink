use std::sync::Arc;
use log::{error, info};
use tokio::sync::mpsc;
use vlink_core::proto::pb::abi::PeerExtraTransport;
use vlink_tun::device::peer::Peer;
use vlink_tun::InboundResult;
use crate::transport::proto::nat_tcp::{NatTcpTransportClient, NatTcpTransportParam};
use crate::transport::proto::nat_udp::NatUdpTransportClient;

/// 扩展传输层选择器
/// 选择扩展协议，更新peer的endpoint

pub struct ExtTransportSelector {
    peer: Arc<Peer>,
    transports: Vec<PeerExtraTransport>,
}

impl ExtTransportSelector {
    pub fn new(peer: Arc<Peer>, inbound_tx: mpsc::Sender<InboundResult>, transports: Vec<PeerExtraTransport>) -> Self {
        let peer_c = Arc::clone(&peer);
        let transports_c = transports.clone();
        let inbound_tx_c = inbound_tx.clone();
        tokio::spawn(async move {
            if peer_c.endpoint.read().unwrap().is_none() {
                //启动对应的传输层连接器

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
                            match NatTcpTransportClient::new(peer_c.clone(), inbound_tx_c, e.endpoint.clone()).await {
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
                //NatUdpTransportClient
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