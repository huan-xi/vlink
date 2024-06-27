use std::fmt::{Debug, Display, Formatter};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use igd::PortMappingProtocol;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use vlink_tun::{InboundResult, OutboundSender};
use vlink_tun::device::event::{DeviceEvent, DevicePublisher, ExtraEndpoint};
use vlink_tun::device::peer::Peer;

use crate::transport::forward::udp::UdpForwarder;
use crate::transport::nat2pub::nat_service::{NatService, NatServiceParam};
use crate::transport::sender::udp_sender::Ipv4UdpOutboundSender;

pub const PROTO_NAME: &str = "NatUdp";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatUdpTransportParam {
    stun_servers: Vec<String>,
    /// 0 自动获取
    nat_port: u16,
}

/// 第一种方案
/// nat1 udp 传输
/// 通过nat1 穿透wire的wireguard udp 端口, 对端可以直接用wireguard 标准协议直连
/// 第二种方案
/// 直接设置wireguard udp成端口复用，用udp端口连接stun 服务器，获取公网地址，直接就穿透成功,
/// 不需要端口转发
///
/// 第三种方案，直接转到device 的数据接收器

/// 默认流程
/// b监听udp,a监听udp
/// ab 同时发送握手
/// 假设a先收到b的握手包,更新a to b endpoint
/// b收到a的握手包更新,更新b to a endpoint



/// b监听nat1 udp端口报告服务器,并开启udp数据转发器->将udp数据转到tun设备，将接受到的数据转到设备,作为服务端不主动发起连接
/// a收到b nat1udp 端口,开启udp数据转发器,也是将数据转到tun设备
/// a收到to b endpoint后发起握手
/// b收到a握手包,更新 b to a endpoint
/// ab 握手成功根据对方的endpoint 交换数据



pub struct NatUdpTransport {
    svc: NatService,
    sender: Sender<InboundResult>,
    forwarder: Option<UdpForwarder>,
    event_pub: DevicePublisher,
}

impl NatUdpTransport {
    pub async fn new(sender: Sender<InboundResult>,
                     param: NatUdpTransportParam,
                     event_pub: DevicePublisher) -> anyhow::Result<Self> {
        let svc = NatService::new(NatServiceParam {
            stun_servers: param.stun_servers,
            port: param.nat_port,
            protocol: PortMappingProtocol::UDP,
            upnp_broadcast_address: None,
        });
        Ok(Self {
            sender,
            svc,
            forwarder: None,
            event_pub,
        })
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        //启动nat 服务
        let mut rx = self.svc.start().await?;
        loop {
            if let Some(addr) = rx.recv().await {
                let port = addr.port();
                // let wireguard_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port));
                let forward = UdpForwarder::spawn(self.sender.clone(), port).await?;
                self.forwarder = Some(forward);
                //发送更新端点事件
                let _ = self.event_pub.send(DeviceEvent::ExtraEndpointSuccess(ExtraEndpoint {
                    proto: PROTO_NAME.to_string(),
                    endpoint: addr.to_string(),
                }));
            }
        }
        Ok(())
    }
}

pub struct NatUdpTransportClient {
    dst: SocketAddr,
    socket: Arc<UdpSocket>,

}

/// 客户端
impl NatUdpTransportClient {
    pub async fn new(peer: Arc<Peer>, inbound_tx: mpsc::Sender<InboundResult>, endpoint: String) -> anyhow::Result<Self> {
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
        let socket = UdpSocket::bind(local_addr).await?;
        info!("bind udp socket:{}", socket.local_addr()?);
        let socket = Arc::new(socket);
        // let socket = Arc::new(make_udp_socket(local_addr)?);
        let dst: SocketAddr = endpoint.parse()?;
        // 接受数据
        let socket_c = socket.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            loop {
                match socket_c.recv_from(&mut buf).await {
                    Ok((n, addr)) => {
                        debug!("recv from {},data:{n},dst:{dst}", addr);
                        let data = buf[..n].to_vec();
                        // 将数据转到设备
                        let _ = inbound_tx.send((data, Box::new(Ipv4UdpOutboundSender {
                            dst: addr,
                            socket: socket_c.clone(),
                        }))).await;
                    }
                    Err(e) => {
                        debug!("udp recv error: {:?}", e);
                        break;
                    }
                }
            }
        });


        Ok(Self {
            dst,
            socket,
        })
    }
    pub fn endpoint(&self) -> Box<dyn OutboundSender> {
        Box::new(Ipv4UdpOutboundSender {
            dst: self.dst,
            socket: self.socket.clone(),
        }) as Box<dyn OutboundSender>
    }
}


#[cfg(test)]
pub mod test {
    use std::env;
    use std::net::SocketAddr;
    use std::time::Duration;

    use log::info;
    use tokio::net::UdpSocket;
    use tokio::time;

    use vlink_tun::device::event::DeviceEvent;
    use vlink_tun::InboundResult;

    use crate::transport::proto::nat_udp::{NatUdpTransport, NatUdpTransportParam};

    #[tokio::test]
    pub async fn test_send() -> anyhow::Result<()> {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();
        /* let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0));
         let udp = UdpSocket::bind(local_addr).await?;


         let target = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127,0,0,1), 5523));
         udp.connect(target).await?;*/

        let sock = UdpSocket::bind("0.0.0.0:8080".parse::<SocketAddr>().unwrap()).await?;
        // let remote_addr = "192.168.3.1:5523".parse::<SocketAddr>().unwrap();
        let remote_addr = "175.9.140.13:5524".parse::<SocketAddr>().unwrap();
        sock.connect(remote_addr).await?;

        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            let _ = sock.send(b"hello").await;
            info!("send hello");
            interval.tick().await;
        }
    }

    #[tokio::test]
    pub async fn test() -> anyhow::Result<()> {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();
        let param = NatUdpTransportParam {
            stun_servers: vec![],
            nat_port: 5524,
        };

        //模拟 wireguard
        /* let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port));
         let socket = UdpSocket::bind(local_addr).await?;
         tokio::spawn(async move {
             let mut buf = [0u8; 1500];
             loop {
                 match socket.recv_from(&mut buf).await {
                     Ok((size, addr)) => {
                         let data = &buf[..size];
                         let str = String::from_utf8_lossy(data);
                         info!("recv from {},data:{}", addr,str);
                         socket.send_to(&buf[..size], addr).await.unwrap();
                     }
                     Err(e) => {
                         error!("recv error: {:?}", e);
                     }
                 }
             }
         });*/
        let (tx, mut rx) = tokio::sync::mpsc::channel::<InboundResult>(1024);
        tokio::spawn(async move {
            loop {
                let (data, endpoint) = rx.recv().await.unwrap();
                info!("recv from {},data:{}", endpoint, String::from_utf8_lossy(&data));
                tokio::spawn(async move {
                    let mut interval = time::interval(Duration::from_secs(1));
                    let mut count = 0;
                    loop {
                        let _ = endpoint.send(b"hello".to_vec().as_slice()).await;
                        interval.tick().await;
                        count += 1;
                        if count > 10 {
                            break;
                        }
                    }
                });
            }
        });
        //broadcast::Sender<DeviceEvent>;
        let (tx1, rx1) = tokio::sync::broadcast::channel::<DeviceEvent>(1024);

        // time::sleep(Duration::from_secs(100000)).await;
        let mut a = NatUdpTransport::new(tx, param, tx1).await.unwrap();

        a.start().await?;
        info!("{}", a);
        Ok(())
    }
}

impl Display for NatUdpTransport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NatUdpTransport")
    }
}

impl Debug for NatUdpTransport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NatUdpTransport")
    }
}