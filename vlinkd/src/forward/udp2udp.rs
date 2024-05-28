use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use futures_util::future::select;
use log::{error, info};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use crate::transport::nat2pub::reuse_socket::make_udp_socket;

/// 端口装发
///
pub struct UdpToUdpForwarder {
    token: CancellationToken,
}

impl Drop for UdpToUdpForwarder {
    fn drop(&mut self) {
        self.token.cancel()
    }
}

impl UdpToUdpForwarder {
    pub async fn spawn(local_port: u16, target: SocketAddr) -> anyhow::Result<Self> {
        info!("start udp forwarder,local_port:{},target:{}", local_port, target);
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), local_port));
        let socket = make_udp_socket(local_addr)?;
        // let (tx1, rx1) = mpsc::channel(1024);
        let token = CancellationToken::new();
        let h1 = async move {
            let mut buf = vec![0u8; 1024];
            loop {
                let (n, addr) = match socket.recv_from(&mut buf).await {
                    Ok(e) => e,
                    Err(e) => {
                        error!("recv error:{}", e);
                        break;
                    }
                };
                info!("recv {} bytes:{addr}", n);
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
        })
    }
}