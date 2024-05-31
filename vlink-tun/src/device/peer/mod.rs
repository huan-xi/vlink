pub mod cidr;
pub(crate) mod peers;
pub(crate) mod session;
mod monitor;
pub(crate) mod handler;
mod handshake;
mod inbound;

use std::fmt::{Display, Formatter};
use std::sync::RwLock;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use log::{debug, warn};
use tokio_util::sync::CancellationToken;
use crate::{NativeTun, PeerStaticSecret, Tun};
use crate::device::event;
use crate::device::event::DeviceEvent;
use crate::device::inbound::OutboundSender;
use crate::noise::handshake::IncomingInitiation;
use crate::noise::{crypto, protocol};
use crate::device::peer::handshake::Handshake;
use crate::device::peer::monitor::{PeerMetrics, PeerMonitor};
use crate::device::peer::session::{ActiveSession, Session, SessionIndex};
use crate::noise::crypto::PublicKey;

#[derive(Debug)]
pub(crate) enum OutboundEvent {
    Data(Vec<u8>),
}

#[derive(Debug)]
pub(crate) enum InboundEvent
{
    HanshakeInitiation {
        endpoint: Box<dyn OutboundSender>,
        initiation: IncomingInitiation,
    },
    HandshakeResponse {
        endpoint: Box<dyn OutboundSender>,
        packet: protocol::HandshakeResponse,
        session: Session,
    },
    CookieReply {
        endpoint: Box<dyn OutboundSender>,
        packet: protocol::CookieReply,
        session: Session,
    },
    TransportData {
        endpoint: Box<dyn OutboundSender>,
        packet: protocol::TransportData,
        session: Session,
    },
}

pub(crate) type InboundTx = mpsc::Sender<InboundEvent>;
pub(crate) type InboundRx = mpsc::Receiver<InboundEvent>;
pub(crate) type OutboundTx = mpsc::Sender<OutboundEvent>;
pub(crate) type OutboundRx = mpsc::Receiver<OutboundEvent>;

pub struct WatchOnline {
    online_rx: watch::Receiver<bool>,
    online_tx: watch::Sender<bool>,
}


impl WatchOnline {
    pub fn new(init: bool) -> Self {
        let (online_tx, online_rx) = watch::channel(init);
        Self {
            online_rx,
            online_tx,
        }
    }
}

/// 通过endpoint 发送数据
/// udp-> peer
pub struct Peer {
    pub_key: PublicKey,
    tun: NativeTun,
    online: WatchOnline,
    monitor: PeerMonitor,
    handshake: RwLock<Handshake>,
    pub sessions: RwLock<ActiveSession>,
    /// 连接端点, 用于发送数据
    pub endpoint: RwLock<Option<Box<dyn OutboundSender>>>,
    inbound: InboundTx,
    outbound: OutboundTx,
    ip_addr: String,
    event_pub: event::DevicePublisher,

    /// 可以取消和peer 相关的任务
    token: CancellationToken,
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

impl Peer {
    pub(super) fn new(
        tun: NativeTun,
        secret: PeerStaticSecret,
        session_index: SessionIndex,
        endpoint: Option<Box<dyn OutboundSender>>,
        inbound: InboundTx,
        outbound: OutboundTx,
        persitent_keepalive_interval: Option<Duration>,
        is_online: bool,
        ip_addr: String,
        event_pub: event::DevicePublisher,
    ) -> Self {
        let handshake = RwLock::new(Handshake::new(secret.clone(), session_index.clone()));
        let sessions = RwLock::new(ActiveSession::new(session_index));
        let monitor = PeerMonitor::new(persitent_keepalive_interval);
        let endpoint = RwLock::new(endpoint);
        Self {
            pub_key: secret.public_key().clone(),
            tun,
            handshake,
            sessions,
            inbound,
            outbound,
            endpoint,
            monitor,
            online: WatchOnline::new(is_online),
            ip_addr,
            event_pub,
            token: Default::default(),
        }
    }
    pub fn child_token(&self) -> CancellationToken {
        self.token.child_token()
    }
    pub fn set_online(&self, val: bool) {
        if let Err(e) = self.online.online_tx.send(val)
        {
            warn!("{} not able to set online: {}", self, e);
        }
    }
    #[inline]
    pub async fn await_online(&self) {
        if !*self.online.online_rx.borrow() {
            let mut rx = self.online.online_rx.clone();
            while let Ok(()) = rx.changed().await {
                if *rx.borrow() {
                    return;
                }
            }
        }
    }

    /// Stage inbound data from tun.
    #[inline]
    pub async fn stage_inbound(&self, e: InboundEvent) {
        if let Err(e) = self.inbound.send(e).await {
            warn!("{} not able to handle inbound: {}", self, e);
        }
    }

    pub fn pub_event(&self, event: DeviceEvent) {
        let _ = self.event_pub.send(event);
    }

    /// Stage outbound data to be sent to the peer
    #[inline]
    pub async fn stage_outbound(&self, buf: Vec<u8>) {
        if let Err(e) = self.outbound.send(OutboundEvent::Data(buf)).await {
            warn!("{} not able to stage outbound: {}", self, e);
        }
    }
    #[inline]
    pub fn metrics(&self) -> PeerMetrics {
        self.monitor.metrics()
    }
    #[inline]
    pub async fn keepalive(&self) {
        if !self.monitor.keepalive().can(self.monitor.traffic()) {
            debug!("{self} not able to send keepalive");
            return;
        }
        self.monitor.keepalive().attempt();
        debug!("{self} sending keepalive");
        self.stage_outbound(vec![]).await;
    }

    #[inline]
    async fn send_outbound(&self, buf: &[u8]) {
        //发送wg数据包到节点
        let endpoint = {
            self.endpoint.read().unwrap().as_ref().map(|e| e.box_clone())
        };
        if let Some(mut endpoint) = endpoint {
            if let Err(e) = endpoint.send(buf).await {
                warn!("{} not able to send outbound: {}", self, e);
            }
        } else {
            debug!("no endpoint to send outbound packet to peer {self}");
        }
    }
    #[inline]
    pub fn update_endpoint(&self, endpoint: Box<dyn OutboundSender>) {
        let mut guard = self.endpoint.write().unwrap();
        let _ = guard.insert(endpoint);
    }
}

impl Display for Peer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Peer({}),endpoint:{:?}", self.ip_addr, self.endpoint.read())
    }
}
