use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::net::SocketAddr;
use std::sync::Mutex;
use async_trait::async_trait;
use tokio::sync::{mpsc};
use crate::device::transport::udp::{UdpOutboundSender, UdpSocketInfo, UdpTransport};


#[async_trait]
pub trait OutboundSender: BoxCloneOutboundSender + Send + Sync + Debug + Display {
    async fn send(&self, data: &[u8]) -> Result<(), io::Error>;
    fn dst(&self) -> SocketAddr;

    fn protocol(&self) -> String;

    fn writeable(&self) -> bool {
        true
    }
}




pub trait BoxCloneOutboundSender {
    fn box_clone(&self) -> Box<dyn OutboundSender>;
}

/// 需要接受数据和数据发送器
/// 数据和发送器
pub type InboundResult = (Vec<u8>, Box<dyn OutboundSender>);

/// 设备的数据入口
pub(super) struct Inbound {
    // ///传输层列表,udp,tcp,nat_udp,nat_tcp,derp,
    // pub(crate) transport: TransportDispatcher,
    pub(crate) tx: mpsc::Sender<InboundResult>,
    rx: Mutex<Option<mpsc::Receiver<InboundResult>>>,
    socket_info: UdpSocketInfo,
}

impl Inbound {
    pub fn new(tx: mpsc::Sender<InboundResult>,
               rx: mpsc::Receiver<InboundResult>,
               socket_info: UdpSocketInfo) -> Self {
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            socket_info,
        }
    }
    pub fn tx(&self) -> mpsc::Sender<InboundResult> {
        self.tx.clone()
    }
    pub fn take_rx(&self) -> Option<mpsc::Receiver<InboundResult>> {
        self.rx.lock().unwrap().take()
    }
    pub fn endpoint_for(&self, addr: SocketAddr) -> Box<dyn OutboundSender> {
        Box::new(UdpOutboundSender {
            dst: addr,
            ipv4: self.socket_info.ipv4.clone(),
            ipv6: self.socket_info.ipv6.clone(),
        })
    }
}