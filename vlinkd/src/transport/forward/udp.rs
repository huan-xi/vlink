use std::fmt::{Debug, Display};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use log::{debug, error, info, warn};
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use vlink_tun::{InboundResult, OutboundSender};

use crate::transport::nat2pub::reuse_socket::make_udp_socket;
use crate::transport::sender::udp_sender::Ipv4UdpOutboundSender;

/// udp 转发
///
pub struct UdpForwarder {
    token: CancellationToken,
    sender: Sender<InboundResult>,
}
impl Drop for UdpForwarder {
    fn drop(&mut self) {
        self.token.cancel()
    }
}

impl UdpForwarder {
    pub async fn spawn(sender: Sender<InboundResult>, local_port: u16) -> anyhow::Result<Self> {
        info!("start udp forwarder,local_port:{}", local_port);
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, local_port));
        let socket = Arc::new(make_udp_socket(local_addr)?);
        let token = CancellationToken::new();
        let tx = sender.clone();
        let socket_c = socket.clone();
        let forward_handler = async move {
            let mut buf = vec![0u8; 2048];
            loop {
                // socket_c.recv_from();
                let (n, addr) = match socket_c.recv_from(&mut buf).await {
                    Ok(e) => e,
                    Err(e) => {
                        error!("recv error:{}", e);
                        break;
                    }
                };
                debug!("recv {} bytes:{addr}", n);
                let data = buf[..n].to_vec();
                if let Err(e)=tx.send((data, Box::new(Ipv4UdpOutboundSender { dst: addr, socket: socket_c.clone() }))).await{
                    warn!("send to sender error:{}",e);
                }
            }
            //todo 接受失败
        };
        let token_c = token.clone();
        tokio::spawn(async move {
            loop {
                select! {
                _ = forward_handler => {
                    break;
                }
                _ = token_c.cancelled() => {
                    break;
                }
            }
            }
        });

        Ok(Self {
            token,
            sender,
        })
    }
}