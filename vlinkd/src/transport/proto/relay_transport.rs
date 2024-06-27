use std::collections::hash_map::Entry;
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use futures_util::StreamExt;
use log::{debug, error};
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tailscale_derp::derp_client::DerpClient;
use tailscale_derp::{DerpRequest, DerpResponse};
use vlink_core::base64::{decode_base64, decode_base64_key, encode_base64};
use vlink_core::proto::pb::abi::{peer_forward, PeerForward, RequireReply};
use vlink_core::proto::pb::abi::to_client::ToClientData;
use vlink_core::rw_map::RwMap;
use vlink_tun::{BoxCloneOutboundSender, InboundResult, OutboundSender, PeerList};
use vlink_tun::device::event;
use crate::client::VlinkClient;

pub struct DerpTask {
    token: CancellationToken,
    client: DerpClient,
}

/// 中继传输层
/// 协商最快的中继服务器
/// 启动中继服务，替换掉两个peer的endpoint
///
/// 握手过程,
#[derive(Clone)]
pub struct RelayTransport {
    ///中继服务器列表
    derp_client_map: RwMap<String, DerpTask>,
    client: Arc<VlinkClient>,
    inbound_sender: mpsc::Sender<InboundResult>,
    peers: Arc<RwLock<PeerList>>,
}


impl RelayTransport {
    // 启动中继传输层
    pub fn spawn(cc: Arc<VlinkClient>, sender: mpsc::Sender<InboundResult>,
                 peers: Arc<RwLock<PeerList>>, bus: event::DevicePublisher, ) -> Self {
        let mut tx = cc.subscribe();
        let client_map = RwMap::new();
        let client = cc.clone();
        let sender_c = sender.clone();

        let t = RelayTransport {
            derp_client_map: client_map,
            client,
            inbound_sender: sender_c,
            peers,
        };
        let tt = t.clone();
        tokio::spawn(async move {
            while let Ok(data) = tx.recv().await {
                //请求连接中继服务器,
                if let Some(ToClientData::RequireReply(data)) = data.to_client_data {
                    //来源key
                    let src_key = decode_base64_key(data.src.as_str());
                    if let Err(e) = tt.connect_derp_server(data.server.clone(), src_key).await {
                        error!("connect derp server error:{:?}", e);
                    }
                }
            }
        });
        t
    }

    ///选择最近的中继服务器,要求目标连接该服务器

    pub async fn require_reply(&self, target_pub: &[u8; 32]) -> anyhow::Result<()> {
        let server = "wss://oaodev.local.hperfect.cn:11443/derp".to_string();
        if let Err(e) = self.connect_derp_server(server.clone(), target_pub.clone()).await {
            error!("connect derp server error:{:?}", e);
        }

        let target_pub_key = encode_base64(target_pub);
        let src = self.client.secret.base64_pub();
        let _ = self.client.forward_to(target_pub_key, peer_forward::Data::RequireReply(RequireReply {
            src,
            proto: "".to_string(),
            server: server.clone(),
        })).await;
        Ok(())
    }

    ///连接中继服务器
    /// pub_key 本机公钥
    pub async fn connect_derp_server(&self, server: String, target: [u8; 32]) -> anyhow::Result<mpsc::Sender<DerpRequest>> {
        let key = self.client.secret.private_key.as_bytes().clone();
        let tx = match self.derp_client_map.write_lock().await.entry(server.clone()) {
            Entry::Occupied(e) => {
                debug!("server {server} is connected");
                Ok(e.get().client.sender.clone())
            }
            Entry::Vacant(vacant) => {
                //来源key
                //检测是否连接
                return match DerpClient::new(key, server.as_str()).await {
                    Ok(mut derp_cli) => {
                        let tx = derp_cli.sender.clone();
                        match derp_cli.async_connect().await {
                            Ok((mut stream)) => {
                                let token = self.client.token.child_token();
                                //连接成功
                                vacant.insert(DerpTask {
                                    token: token.clone(),
                                    client: derp_cli,
                                });
                                //启动数据交换,derp 服务器->tun
                                let inbound = self.inbound_sender.clone();
                                let txc = tx.clone();
                                let c = async move {
                                    while let Some(Ok(resp)) = stream.next().await {
                                        match resp {
                                            DerpResponse::FrameRecvPacket((src, data)) => {
                                                // debug!("recv data from derp:{:?}", encode_base64(&data));
                                                let _ = inbound.send((data, Box::new(TailscleDerpOutboundSender {
                                                    dst: src,
                                                    sender: tx.clone(),
                                                }))).await;
                                            }
                                            DerpResponse::FramePeerGonePacket((src, reason)) => {
                                                //todo 处理目标节点断开
                                            }
                                            _ => {}
                                        }
                                    };
                                    // 连接断开
                                    //todo 发送传输层中断请求
                                    //peer.clear_endpoint();
                                };

                                tokio::spawn(async move {
                                    loop {
                                        select! {
                                                _ = c => {
                                                    break;
                                                }
                                                _ = token.cancelled() => {
                                                    break;
                                                }
                                            }
                                    }
                                });
                                Ok(txc)
                            }
                            Err(e) => {
                                error!("connect derp server error:{:?}", e);
                                Err(anyhow::anyhow!("connect derp server error:{:?}", e))
                            }
                        }
                    }
                    Err(e) => {
                        error!("create derp client error:{:?}", e);
                        Err(anyhow::anyhow!("create derp client error:{:?}", e))
                    }
                };
            }
        };

        if let Ok(txc) = tx.as_ref().cloned() {
            //替换endpoint
            if let Some(peer) = self.peers.read().unwrap().get_by_key(&target) {
                //替换endpoint
                let e = Box::new(TailscleDerpOutboundSender {
                    dst: target,
                    sender: txc.clone(),
                });
                let _ = peer.update_endpoint(e);
            }
        }
        return tx;
    }
}


#[derive(Clone)]
pub struct TailscleDerpOutboundSender {
    dst: [u8; 32],
    pub sender: mpsc::Sender<DerpRequest>,
}

impl BoxCloneOutboundSender for TailscleDerpOutboundSender {
    fn box_clone(&self) -> Box<dyn OutboundSender> {
        Box::new(self.clone())
    }
}

impl Debug for TailscleDerpOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for TailscleDerpOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "tailscale dst:{:?}", encode_base64(&self.dst))
    }
}

#[async_trait]
impl OutboundSender for TailscleDerpOutboundSender {
    async fn send(&self, data: &[u8]) -> Result<(), std::io::Error> {
        debug!("send data to tailscale:{:?}", encode_base64(data));
        self.sender.send(DerpRequest::SendPacket((self.dst.to_vec(), data.to_vec()))).await
            .map_err(|e| std::io::Error::new(ErrorKind::NotConnected, e))?;
        Ok(())
    }

    fn dst(&self) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
    }

    fn protocol(&self) -> String {
        "Reply".to_string()
    }
}