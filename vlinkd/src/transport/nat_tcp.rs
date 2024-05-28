use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use igd::PortMappingProtocol;
use log::info;
use serde::{Deserialize, Serialize};
use crate::forward::tcp2udp::TcpToUdpForwarder;
use crate::forward::udp2udp::UdpToUdpForwarder;
use crate::transport::nat2pub::nat_service::{NatService, NatServiceParam};
use crate::transport::nat_udp::NatUdpTransportParam;

pub struct NatTcpTransport {
    wireguard_port: u16,
    svc: NatService,
    forwarder: Option<TcpToUdpForwarder>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatTcpTransportParam {
    stun_servers: Vec<String>,
    wireguard_port: u16,
    /// 0 自动获取
    nat_port: u16,
}

impl NatTcpTransport {
    pub async fn new(param: NatTcpTransportParam) -> anyhow::Result<Self> {
        let svc = NatService::new(NatServiceParam {
            stun_servers: param.stun_servers,
            port: param.nat_port,
            protocol: PortMappingProtocol::TCP,
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
                info!("start forward: {} -> {}", port, wireguard_addr);
                let forward = TcpToUdpForwarder::spawn(port, wireguard_addr).await?;
                self.forwarder = Some(forward);
            }
        }
    }
}

#[cfg(test)]
pub mod test{
    use std::env;
    use std::time::Duration;
    use crate::transport::nat_tcp::{NatTcpTransport, NatTcpTransportParam};
    use crate::transport::nat_udp::NatUdpTransportParam;

    #[tokio::test]
    pub async fn test() -> anyhow::Result<()> {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();
        let param = NatTcpTransportParam {
            stun_servers: vec![],
            wireguard_port: 5523,
            nat_port: 5524,
        };
        let port = param.wireguard_port;
        let mut a = NatTcpTransport::new(param).await?;
        a.start().await?;

        tokio::time::sleep(Duration::from_secs(10000)).await;
        Ok(())
    }
}