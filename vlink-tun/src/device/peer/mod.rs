pub mod cidr;
pub(crate) mod peers;
pub(crate) mod session;
mod monitor;
pub(crate) mod handler;
mod handshake;
mod inbound;

use std::fmt::{Display, Formatter};
use std::sync::{Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use log::{debug, warn};
use crate::{NativeTun, PeerStaticSecret,  Tun};
use crate::noise::handshake::IncomingInitiation;
use crate::noise::{crypto, protocol};
use crate::device::peer::handshake::Handshake;
use crate::device::peer::monitor::{PeerMetrics, PeerMonitor};
use crate::device::peer::session::{ActiveSession, Session, SessionIndex};
use crate::device::endpoint::Endpoint;

#[derive(Debug)]
pub(crate) enum OutboundEvent {
    Data(Vec<u8>),
}

#[derive(Debug)]
pub(crate) enum InboundEvent
{
    HanshakeInitiation {
        endpoint: Endpoint,
        initiation: IncomingInitiation,
    },
    HandshakeResponse {
        endpoint: Endpoint,
        packet: protocol::HandshakeResponse,
        session: Session,
    },
    CookieReply {
        endpoint: Endpoint,
        packet: protocol::CookieReply,
        session: Session,
    },
    TransportData {
        endpoint: Endpoint,
        packet: protocol::TransportData,
        session: Session,
    },
}

pub(crate) type InboundTx = mpsc::Sender<InboundEvent>;
pub(crate) type InboundRx = mpsc::Receiver<InboundEvent>;
pub(crate) type OutboundTx = mpsc::Sender<OutboundEvent>;
pub(crate) type OutboundRx = mpsc::Receiver<OutboundEvent>;

pub struct Peer {
    tun: NativeTun,
    monitor: PeerMonitor,
    handshake: RwLock<Handshake>,
    sessions: RwLock<ActiveSession>,
    // 连接端点
    endpoint: RwLock<Option<Endpoint>>,
    inbound: InboundTx,
    outbound: OutboundTx,
}

impl Peer {
    pub(super) fn new(
        tun: NativeTun,
        secret: PeerStaticSecret,
        session_index: SessionIndex,
        endpoint: Option<Endpoint>,
        inbound: InboundTx,
        outbound: OutboundTx,
        persitent_keepalive_interval: Option<Duration>,
    ) -> Self {
        let handshake = RwLock::new(Handshake::new(secret.clone(), session_index.clone()));
        let sessions = RwLock::new(ActiveSession::new(session_index));
        let monitor = PeerMonitor::new(persitent_keepalive_interval);
        let endpoint = RwLock::new(endpoint);
        Self {
            tun,
            handshake,
            sessions,
            inbound,
            outbound,
            endpoint,
            monitor,
        }
    }

    /// Stage inbound data from tun.
    #[inline]
    pub async fn stage_inbound(&self, e: InboundEvent) {
        if let Err(e) = self.inbound.send(e).await {
            warn!("{} not able to handle inbound: {}", self, e);
        }
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
            self.endpoint.read().unwrap().clone()
        };
        if let Some(mut endpoint) = endpoint {
            endpoint.send(buf).await.unwrap();
        } else {
            debug!("no endpoint to send outbound packet to peer {self}");
        }
    }
    #[inline]
    pub fn update_endpoint(&self, endpoint:Endpoint ) {
        let mut guard = self.endpoint.write().unwrap();
        let _ = guard.insert(endpoint);
    }

}

impl Display for Peer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        //             "Peer({})",
//             crypto::encode_to_hex(self.secret.public_key().as_bytes())
        write!(f, "Peer")
    }
}