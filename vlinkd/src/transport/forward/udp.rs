use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use async_trait::async_trait;
use futures_util::future::select;
use log::{debug, error, info};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use vlink_tun::device::transport::udp::UdpOutboundSender;
use vlink_tun::{BoxCloneOutboundSender, InboundResult, OutboundSender};
use crate::transport::nat2pub::reuse_socket::make_udp_socket;

/// udp 转发
///
pub struct UdpForwarder {
    token: CancellationToken,
    sender: Sender<InboundResult>,
}

#[derive(Clone)]
struct UdpForwarderOutboundSender {
    dst: SocketAddr,
    ipv4: Arc<UdpSocket>,
}

impl BoxCloneOutboundSender for UdpForwarderOutboundSender {
    fn box_clone(&self) -> Box<dyn OutboundSender> {
        Box::new(self.clone())
    }
}

impl Debug for UdpForwarderOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for UdpForwarderOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
#[async_trait]
impl OutboundSender for UdpForwarderOutboundSender {
    async fn send(&self, data: &[u8]) -> Result<(), Error> {
        self.ipv4.send_to(data, self.dst).await?;
        Ok(())
    }

    fn dst(&self) -> SocketAddr {
        self.dst
    }
}

impl Drop for UdpForwarder {
    fn drop(&mut self) {
        self.token.cancel()
    }
}

impl UdpForwarder {
    pub async fn spawn(sender: Sender<InboundResult>, local_port: u16, target: SocketAddr) -> anyhow::Result<Self> {
        info!("start udp forwarder,local_port:{},target:{}", local_port, target);
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), local_port));
        let socket = Arc::new(make_udp_socket(local_addr)?);
        // let (tx1, rx1) = mpsc::channel(1024);
        let token = CancellationToken::new();
        let tx = sender.clone();
        let socket_c = socket.clone();
        let h1 = async move {
            let mut buf = vec![0u8; 1024];
            loop {
                let (n, addr) = match socket_c.recv_from(&mut buf).await {
                    Ok(e) => e,
                    Err(e) => {
                        error!("recv error:{}", e);
                        break;
                    }
                };
                debug!("recv {} bytes:{addr}", n);
                let data = buf[..n].to_vec();
                tx.send((data, Box::new(UdpForwarderOutboundSender { dst: addr, ipv4: socket_c.clone() }))).await.unwrap();
                //查询addr 对应的socket,发往socket，如果没有则创建
                //tx1.send(&buf[])
            }
        };
        let token_c = token.clone();
        tokio::spawn(async move {
            loop {
                select! {
                _ = h1 => {
                    break;
                }
                _ = token_c.cancelled() => {
                    break;
                }
            }
            }
        });
        /*
           let socket = UdpSocket::bind("0.0.0.0:0").await?;
           socket.connect(target).await?;*/


        Ok(Self {
            token,
            sender,
        })
    }
}