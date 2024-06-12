use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use vlink_tun::device::event::{DeviceEvent, DevicePublisher, ExtraEndpoint};
use vlink_tun::InboundResult;

use crate::transport::nat2pub::reuse_socket::make_tcp_socket;
use crate::transport::proto::nat_tcp;
use crate::transport::proto::nat_tcp::{NatTcpTransport, TcpOutboundSender};

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
    pub async fn spawn(local_port: u16, event_pub: DevicePublisher, sender: Sender<InboundResult>) -> anyhow::Result<Self> {
        info!("start tcp forwarder,local_port:{}", local_port);
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, local_port));
        let token = CancellationToken::new();
        let socket = make_tcp_socket(local_addr)?;
        let listener = socket.listen(1024)?;
        let event_pub_c = event_pub.clone();
        let handler = async move {
            if let Err(e) = handler0(listener, event_pub_c, sender).await {
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

/// 将数据转发至inbound
pub async fn handler0(listener: TcpListener, event_pub: DevicePublisher, inbound: Sender<InboundResult>) -> anyhow::Result<()> {
    loop {
        // let (stream, addr) = listener.accept().await;
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                info!("accept tcp stream:{}", addr);
                let inbound_c = inbound.clone();
                let event_pub= event_pub.clone();
                //将数据转发至inbound
                tokio::spawn(async move {
                    // let inbound_cc = inbound_c.clone();
                    let (mut x, v) = stream.into_split();
                    let tcp_sender = TcpOutboundSender {
                        dst: addr,
                        writer: Arc::new(Mutex::new(v)),
                        // is_closed: AtomicBool::new(false),
                    };
                    let mut buf = vec![0u8; 2048];
                    loop {
                        if let Ok(n) = x.read(&mut buf).await {
                            if n <= 0 {
                                break;
                            };
                            // info!("read data:{}", String::from_utf8_lossy(&buf[..n]));
                            inbound_c.send((buf[..n].to_vec(), Box::new(tcp_sender.clone()))).await.unwrap();
                        } else {
                            // tcp 断开
                            break;
                        }
                    }
                    let _ = event_pub.send(DeviceEvent::TransportFailed(ExtraEndpoint {
                        proto: nat_tcp::PROTO_NAME.to_string(),
                        endpoint: addr.to_string(),
                    }));
                    //tcp 断开
                });
            }
            Err(e) => {}
        }
    }

    Ok(())
}



