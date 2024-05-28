use std::fmt::{Debug, Display, Formatter};
use std::io::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use async_trait::async_trait;
use igd::PortMappingProtocol;
use serde::{Deserialize, Serialize};
use vlink_tun::device::endpoint::Endpoint;
use vlink_tun::device::transport::Transport;
use crate::forward::udp2udp::UdpToUdpForwarder;
use crate::transport::nat2pub::nat_service::{NatService, NatServiceParam};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatUdpTransportParam {
    stun_servers: Vec<String>,
    wireguard_port: u16,
    /// 0 自动获取
    nat_port: u16,
}

/// 第一种方案
/// nat1 udp 传输
/// 通过nat1 穿透wire的wireguard udp 端口, 对端可以直接用wireguard 标准协议直连
/// 第二种方案
/// 直接设置wireguard udp成端口复用，用udp端口连接stun 服务器，获取公网地址，直接就穿透成功,
/// 不需要端口转发

pub struct NatUdpTransport {
    wireguard_port: u16,
    svc: NatService,
    forwarder: Option<UdpToUdpForwarder>,
}

impl NatUdpTransport {
    pub async fn new(param: NatUdpTransportParam) -> anyhow::Result<Self> {
        let svc = NatService::new(NatServiceParam {
            stun_servers: param.stun_servers,
            port: param.nat_port,
            protocol: PortMappingProtocol::UDP,
            upnp_broadcast_address: None,
        });
        Ok(Self {
            wireguard_port: param.wireguard_port,
            svc,
            forwarder: None,
        })
    }
    pub async fn start(&mut self) -> anyhow::Result<()> {
        //启动nat 服务
        let mut rx = self.svc.start().await?;
        loop {
            if let Some(addr) = rx.recv().await {
                let port = addr.port();
                let wireguard_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port));
                let forward = UdpToUdpForwarder::spawn(port, wireguard_addr).await?;
                self.forwarder = Some(forward);
            }
        }


        /*        match rx {
                    stun_format::SocketAddr::V4(ipv4, port) => {
                        //监听svc端口绑定, 启动端口代理,监听本机xx端口, 收到数据转发到wiregard 端口
                        let wireguard_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port));
                        let forward = UdpToUdpForwarder::spawn(port, wireguard_addr);
                        self.forwarder = Some(forward);

                        // tokio::spawn()
                        // udp_forward
                        /*let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port));
                        let socket = make_udp_socket(local_addr)?;
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
                    }
                    stun_format::SocketAddr::V6(_, _) => {
                        return Err(anyhow::anyhow!("stun not support ipv6"));
                    }
                };*/
        Ok(())
    }
}


#[async_trait]
impl Transport for NatUdpTransport {
    fn port(&self) -> u16 {
        todo!()
    }

    async fn send_to(&self, data: &[u8], dst: SocketAddr) -> Result<(), Error> {
        todo!()
    }

    async fn recv_from(&mut self) -> Result<(Endpoint, Vec<u8>), Error> {
        todo!()
    }
}

#[cfg(test)]
pub mod test {
    use std::env;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
    use std::time::Duration;
    use log::{error, info};
    use tokio::net::UdpSocket;
    use tokio::time;
    use crate::transport::nat2pub::reuse_socket::make_udp_socket;
    use crate::transport::nat_udp::{NatUdpTransport, NatUdpTransportParam};

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
            wireguard_port: 5523,
            nat_port: 5524,
        };
        let port = param.wireguard_port;

        //模拟 wireguard
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port));
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
        });
        // time::sleep(Duration::from_secs(100000)).await;
        let mut a = NatUdpTransport::new(param).await.unwrap();

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