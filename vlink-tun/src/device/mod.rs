use std::net::{IpAddr, Ipv4Addr};
use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};

use ip_network::IpNetwork;
use log::debug;
use tokio::sync::{broadcast, mpsc, Mutex as AsyncMutex};
use tokio_util::sync::CancellationToken;

use crate::{LocalStaticSecret, Tun};
use crate::device::config::{DeviceConfig, PeerConfig};
use crate::device::handle::DeviceHandle;
use crate::device::inbound::{Inbound, InboundResult};
use crate::device::peer::Peer;
use crate::device::peer::peers::PeerList;
use crate::device::peer::session::Session;
use crate::device::rate_limiter::RateLimiter;
// use crate::device::transport::{Transport, TransportDispatcher, TransportWrapper};
use crate::device::transport::udp::UdpTransport;
use crate::errors::Error;
use crate::noise::handshake::Cookie;
use crate::router::Router;
use crate::tun::IFace;

pub mod peer;
mod handle;

mod metrics;
mod rate_limiter;
mod time;
pub mod config;
pub(crate) mod inbound;
pub mod transport;
pub mod endpoint;
mod crypto;
mod cipher;
pub mod event;

struct Settings
{
    secret: LocalStaticSecret,
    fwmark: u32,
    cookie: Arc<Cookie>,
    inbound: Inbound,
}

impl Settings
{
    pub fn new(inbound: Inbound, private_key: [u8; 32], fwmark: u32) -> Self {
        let secret = LocalStaticSecret::new(private_key);
        let cookie = Arc::new(Cookie::new(&secret));

        Self {
            secret,
            fwmark,
            cookie,
            inbound,
        }
    }
    pub fn secret_and_cookie(&self) -> (LocalStaticSecret, Arc<Cookie>) {
        (self.secret.clone(), self.cookie.clone())
    }

    #[inline(always)]
    pub fn listen_port(&self) -> u16 {
        // self.inbound.port()
        todo!();
    }
}


pub struct Device {
    token: CancellationToken,
    inner: Arc<DeviceInner>,
    handler: DeviceHandle,
    pub port: u16,
}


impl Device {
    /// new 完之后会运行,通过token 手动取消
    /// 启动一个udp 端口
    pub async fn new(name: Option<String>, cfg: DeviceConfig) -> Result<Self, Error> {
        let tun = crate::NativeTun::new(name).map_err(Error::Tun)?;
        tun.enabled(true)?;
        //设置ip,network
        let mask = match cfg.network {
            IpNetwork::V4(n) => n.full_netmask(),
            IpNetwork::V6(n) => todo!("为实现ipv6"),
        };
        debug!("set ip :{};{}",cfg.address,mask);

        // let mask = Ipv4Addr::from(helpers::bite_mask(cfg.netmask));
        // ip_network::Ipv4Network::new(cfg.address, cfg.netmask).unwrap();
        //let network = Ipv4Network::new(cfg.address, cfg.netmask);
        tun.set_ip(cfg.address, mask)?;
        //设置网络路由
        let router = Router::new(tun.name().to_string());

        //Cidr
        router.add_route(cfg.network.network_address(), IpAddr::V4(mask))?;

        let token = CancellationToken::new();
        let (tx, rx) = mpsc::channel::<InboundResult>(1024);
        let (port, socket_info) = UdpTransport::spawn(token.child_token(), cfg.port, tx.clone()).await?;
        let inbound = Inbound::new(tx.clone(), rx, socket_info);
        let settings = Mutex::new(Settings::new(inbound, cfg.private_key, cfg.fwmark));
        let (tx, _) = broadcast::channel(32);
        let peers = Arc::new(RwLock::new(PeerList::new(token.child_token(), tun.clone(), tx.clone())));
        let inner = Arc::new(DeviceInner {
            tun_addr: tun.address()?,
            tun,
            peers,
            settings,
            rate_limiter: RateLimiter::new(u16::MAX),
            event_bus: tx,
        });
        inner.reset_peers(cfg.peers.into_values().collect());

        let handler = DeviceHandle::spawn(token.clone(), inner.clone());
        Ok(Self {
            token,
            inner,
            handler,
            port,
        })
    }

    /// 用于向设备输入数据
    /// 扩展协议可以将数据直接输入到设备
    pub fn inbound_tx(&self) -> mpsc::Sender<InboundResult> {
        self.inner.settings.lock().unwrap().inbound.tx.clone()
    }
}

impl Deref for Device {
    type Target = DeviceInner;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

pub struct DeviceInner {
    pub tun: crate::NativeTun,
    pub tun_addr: Ipv4Addr,
    pub peers: Arc<RwLock<PeerList>>,
    settings: Mutex<Settings>,
    /// 对入口数据限流
    rate_limiter: RateLimiter,
    /// 设备事件总线
    pub event_bus: event::DevicePublisher,
}

impl DeviceInner {
    #[inline]
    pub fn get_peer_by_key(&self, public_key: &[u8; 32]) -> Option<Arc<Peer>> {
        let index = self.peers.read().unwrap();
        index.get_by_key(public_key)
    }

    #[inline]
    pub fn get_session_by_index(&self, i: u32) -> Option<(Session, Arc<Peer>)> {
        let index = self.peers.read().unwrap();
        index.get_session_by_index(i)
    }

    #[inline]
    pub fn reset_peers(&self, peers: Vec<PeerConfig>) {
        let settings = self.settings.lock().unwrap();
        let mut index = self.peers.write().unwrap();
        index.clear();
        for p in peers {
            let mut secret = settings.secret.clone().with_peer(p.public_key);
            if let Some(psk) = p.preshared_key {
                secret.set_psk(psk);
            }

            let endpoint = p.endpoint.map(|addr| settings.inbound.endpoint_for(addr));
            index.insert(secret, p.allowed_ips, endpoint, p.persistent_keepalive, p.is_online, p.ip_addr);
        }
    }
    /// 插入peer 需要确认传输层协议
    #[inline]
    pub fn insert_peer(&self, cfg: PeerConfig) {
        let settings = self.settings.lock().unwrap();
        let mut index = self.peers.write().unwrap();
        let mut secret = settings.secret.clone().with_peer(cfg.public_key);
        if let Some(psk) = cfg.preshared_key {
            secret.set_psk(psk);
        }
        let endpoint = cfg.endpoint.map(|addr| settings.inbound.endpoint_for(addr));
        index.insert(secret, cfg.allowed_ips, endpoint, cfg.persistent_keepalive, cfg.is_online, cfg.ip_addr);
    }
}


#[derive(Clone)]
pub struct DeviceControl
{
    inner: Arc<DeviceInner>,
    handle: Arc<AsyncMutex<DeviceHandle>>,
}