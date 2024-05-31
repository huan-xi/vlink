use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use igd::PortMappingProtocol;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use vlink_tun::device::event::{DeviceEvent, DevicePublisher, ExtraEndpointSuccess};
use vlink_tun::device::peer::Peer;
use vlink_tun::InboundResult;
use crate::client::VlinkClient;
use crate::transport::forward::tcp2udp::TcpForwarder;
use crate::transport::nat2pub::nat_service::{NatService, NatServiceParam};
use crate::transport::proto::nat_udp::{NatUdpTransportParam, PROTO_NAME};

pub struct NatTcpTransport {
    svc: NatService,
    forwarder: Option<TcpForwarder>,
    sender: Sender<InboundResult>,
    event_pub: DevicePublisher
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatTcpTransportParam {
    stun_servers: Vec<String>,
    /// 0 自动获取
    nat_port: u16,

}

impl NatTcpTransport {
    pub async fn new(client: Arc<VlinkClient>, sender: Sender<InboundResult>,
                     param: NatTcpTransportParam, event_pub: DevicePublisher) -> anyhow::Result<Self> {

        let svc = NatService::new(NatServiceParam {
            stun_servers: param.stun_servers,
            port: param.nat_port,
            protocol: PortMappingProtocol::TCP,
            upnp_broadcast_address: None,
        });
        Ok(Self {
            svc,
            forwarder: None,
            sender,
            event_pub
        })
    }
    pub async fn start(&mut self) -> anyhow::Result<()> {
        //启动nat 服务
        let sender_c = self.sender.clone();
        let mut rx = self.svc.start().await?;
        loop {
            if let Some(addr) = rx.recv().await {
                let port = addr.port();
                let forward = TcpForwarder::spawn(port, sender_c.clone()).await?;
                self.forwarder = Some(forward);
                //发送更新端点事件

                let _ = self.event_pub.send(DeviceEvent::ExtraEndpointSuccess(ExtraEndpointSuccess {
                    proto: PROTO_NAME.to_string(),
                    endpoint: addr.to_string(),
                }));
            }
        }
    }
}


pub struct NatTcpTransportClient {

}
impl NatTcpTransportClient{
    pub async fn new(peer: Arc<Peer>, inbound_tx: mpsc::Sender<InboundResult>, endpoint: String) -> anyhow::Result<Self> {
        //建立tcp 连接
        Ok(Self{

        })
    }

    pub fn endpoint(&self){

    }
}

#[cfg(test)]
pub mod test {
    use std::env;
    use std::time::Duration;
    use crate::transport::proto::nat_tcp::{NatTcpTransport, NatTcpTransportParam};
    use crate::transport::proto::nat_udp::NatUdpTransportParam;

    #[tokio::test]
    pub async fn test() -> anyhow::Result<()> {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();
        let param = NatTcpTransportParam {
            stun_servers: vec![],
            nat_port: 5524,
        };
        // let mut a = NatTcpTransport::new(param).await?;
        // a.start().await?;

        tokio::time::sleep(Duration::from_secs(10000)).await;
        Ok(())
    }
}