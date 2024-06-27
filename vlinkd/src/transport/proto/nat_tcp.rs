use std::fmt::{Debug, Display, Formatter};
use std::io::{Error, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use async_trait::async_trait;
use igd::PortMappingProtocol;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpSocket;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::Sender;
use vlink_tun::device::event::{DeviceEvent, DevicePublisher, ExtraEndpoint};
use vlink_tun::device::peer::Peer;
use vlink_tun::{BoxCloneOutboundSender, InboundResult, OutboundSender};
use crate::client::VlinkClient;
use crate::transport::forward::tcp2udp::TcpForwarder;
use crate::transport::nat2pub::nat_service::{NatService, NatServiceParam};

pub const PROTO_NAME: &str = "NatTcp";

pub struct NatTcpTransport {
    svc: NatService,
    forwarder: Option<TcpForwarder>,
    sender: Sender<InboundResult>,
    event_pub: DevicePublisher,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatTcpTransportParam {
    stun_servers: Vec<String>,
    /// 0 自动获取
    nat_port: u16,
}

impl NatTcpTransport {
    pub async fn new(client: Arc<VlinkClient>,
                     sender: Sender<InboundResult>,
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
            event_pub,
        })
    }
    pub async fn start(&mut self) -> anyhow::Result<()> {
        //启动nat 服务
        let sender_c = self.sender.clone();
        let mut rx = self.svc.start().await?;
        let event_pub = self.event_pub.clone();
        loop {
            if let Some(addr) = rx.recv().await {
                let port = addr.port();
                let forward = TcpForwarder::spawn(port,event_pub.clone(), sender_c.clone()).await?;
                self.forwarder = Some(forward);
                //发送更新端点事件
                let _ = self.event_pub.send(DeviceEvent::ExtraEndpointSuccess(ExtraEndpoint {
                    proto: PROTO_NAME.to_string(),
                    endpoint: addr.to_string(),
                }));
            }
        }
    }
}

pub struct NatTcpListener {

}


pub struct NatTcpTransportClient {
    pub sender: TcpOutboundSender<OwnedWriteHalf>,
}

pub struct TcpOutboundSender<T: AsyncWriteExt> {
    pub(crate) dst: SocketAddr,
    pub(crate) writer: Arc<Mutex<T>>,
    // pub is_closed: AtomicBool,
}

impl<T: AsyncWriteExt> Clone for TcpOutboundSender<T> {
    fn clone(&self) -> Self {
        Self {
            dst: self.dst.clone(),
            writer: self.writer.clone(),
        }
    }
}

impl<T: AsyncWriteExt + Send + Sync + Unpin + 'static> BoxCloneOutboundSender for TcpOutboundSender<T> {
    fn box_clone(&self) -> Box<dyn OutboundSender> {
        Box::new(self.clone())
    }
}


impl<T: AsyncWriteExt> Debug for TcpOutboundSender<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl<T: AsyncWriteExt> Display for TcpOutboundSender<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TcpOutboundSender")
    }
}

#[async_trait]
impl<T: AsyncWriteExt + Send + Sync + Unpin + 'static> OutboundSender for TcpOutboundSender<T> {
    async fn send(&self, data: &[u8]) -> Result<(), Error> {
        debug!("write data to tcp");
        self.writer.lock().await.write_all(data).await?;
        Ok(())
    }

    fn dst(&self) -> SocketAddr {
        self.dst
    }

    fn protocol(&self) -> String {
        PROTO_NAME.to_string()
    }
}

impl NatTcpTransportClient {

    /// 启动tcp 服务,并监听
    pub async fn spawn(peer: Arc<Peer>, inbound_tx: mpsc::Sender<InboundResult>, endpoint: String) -> anyhow::Result<Self> {
        //建立tcp 连接
        let addr: SocketAddr = endpoint.clone().parse()?;
        let socket = TcpSocket::new_v4()?;
        let mut stream = socket.connect(addr.clone()).await?;
        let (mut rh, wh) = stream.into_split();
        let awh = Arc::new(Mutex::new(wh));
        let sender = TcpOutboundSender {
            dst: addr,
            writer: awh,
        };
        let sender_c = sender.clone();
        // let tx = Arc::new(tx);
        tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];
            while let Ok(n) = rh.read(&mut buf).await {
                if n <= 0 {
                    break;
                }
                inbound_tx.send((buf[..n].to_vec(), Box::new(sender_c.clone()))).await.unwrap();
            }
            //断开
            *peer.endpoint.write().unwrap() = None;
        });
        Ok(Self {
            sender,
        })
    }

    pub fn endpoint(&self) -> Box<dyn OutboundSender> {
        self.sender.box_clone()
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