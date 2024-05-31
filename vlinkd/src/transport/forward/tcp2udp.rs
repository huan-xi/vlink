use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use vlink_tun::InboundResult;

use crate::transport::nat2pub::reuse_socket::make_tcp_socket;

pub struct TcpForwarder {
    token: CancellationToken,
}

impl Drop for TcpForwarder {
    fn drop(&mut self) {
        self.token.cancel()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatUdpTransportParam {
    stun_servers: Vec<String>,
    wireguard_port: u16,
    /// 0 自动获取
    nat_port: u16,
}

impl TcpForwarder {
    pub async fn spawn(local_port: u16, sender: Sender<InboundResult>) -> anyhow::Result<Self> {
        info!("start tcp forwarder,local_port:{}", local_port);
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, local_port));
        let token = CancellationToken::new();
        let socket = make_tcp_socket(local_addr)?;
        let listener = socket.listen(1024)?;
        let handler = async move {
            if let Err(e) = handler0(listener,sender).await {
                error!("handler error:{}", e);
                return Err(e);
            }
            Ok(())
        };

        let token_c = token.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token_c.cancelled() => {break;}
                    _ = handler => {break;}
                }
            }
        });

        Ok(Self {
            token,
        })
    }
}

pub async fn handler0(listener: TcpListener, sender: Sender<InboundResult>) -> anyhow::Result<()> {
    loop {
        // let (stream, addr) = listener.accept().await;
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                info!("accept tcp stream:{}", addr);
                //将数据转发至udp
                tokio::spawn(async move {
                    let (mut x, v) = stream.split();
                    let mut buf = vec![0u8; 1024];
                    loop {
                        if let Ok(n) = x.read(&mut buf).await {
                            if n <= 0 {
                                break;
                            };
                            info!("read data:{}", String::from_utf8_lossy(&buf[..n]));

                            //sender.send((buf[..n].to_vec(), Box::new(v))).await.unwrap();
                        } else {
                            //todo tcp 断开
                            break;
                        }
                    }
                });
            }
            Err(e) => {}
        }
    }

    Ok(())
}



