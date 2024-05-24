use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use ip_network::{IpNetwork, Ipv4Network};
use tokio_util::sync::CancellationToken;
use crate::{LocalStaticSecret, Tun};
use crate::errors::Error;
use tokio::sync::{Mutex as AsyncMutex};
use log::debug;
use crate::noise::handshake::Cookie;
use crate::device::config::{DeviceConfig, PeerConfig};
use crate::device::handle::DeviceHandle;
use crate::device::inbound::Inbound;
use crate::device::peer::cidr::Cidr;
use crate::device::peer::Peer;
use crate::device::peer::peers::PeerList;
use crate::device::peer::session::Session;
use crate::device::rate_limiter::RateLimiter;
use crate::device::transport::{Transport, TransportDispatcher};
use crate::device::transport::udp::UdpTransport;
use crate::router::{Router};
use crate::tun::IFace;

pub mod peer;
mod handle;

mod metrics;
mod rate_limiter;
mod time;
pub mod config;
mod inbound;
pub mod transport;
pub mod endpoint;
mod crypto;
mod cipher;

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

        // wiretun::Device::with_udp(tun, cfg).await
        let token = CancellationToken::new();
        let transport = UdpTransport::bind(Ipv4Addr::UNSPECIFIED, Ipv6Addr::UNSPECIFIED, cfg.port).await?;
        let port = transport.port();
        let inbound = Inbound {
            transports: vec![TransportDispatcher::Udp(transport)],
        };
        let settings = Mutex::new(Settings::new(inbound, cfg.private_key, cfg.fwmark));
        let peers = Mutex::new(PeerList::new(token.child_token(), tun.clone()));
        let inner = Arc::new(DeviceInner {
            tun_addr: tun.address()?,
            tun,
            peers,
            settings,
            rate_limiter: RateLimiter::new(u16::MAX),
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
    peers: Mutex<PeerList>,
    settings: Mutex<Settings>,
    /// 对入口数据限流
    rate_limiter: RateLimiter,
}

impl DeviceInner {
    #[inline]
    pub fn get_peer_by_key(&self, public_key: &[u8; 32]) -> Option<Arc<Peer>> {
        let index = self.peers.lock().unwrap();
        index.get_by_key(public_key)
    }

    #[inline]
    pub fn get_session_by_index(&self, i: u32) -> Option<(Session, Arc<Peer>)> {
        let index = self.peers.lock().unwrap();
        index.get_session_by_index(i)
    }

    #[inline]
    pub fn reset_peers(&self, peers: Vec<PeerConfig>) {
        let settings = self.settings.lock().unwrap();
        let mut index = self.peers.lock().unwrap();
        index.clear();
        for p in peers {
            let mut secret = settings.secret.clone().with_peer(p.public_key);
            if let Some(psk) = p.preshared_key {
                secret.set_psk(psk);
            }
            let endpoint = p.endpoint.map(|addr| settings.inbound.endpoint_for(addr));
            index.insert(secret, p.allowed_ips, endpoint, p.persistent_keepalive);
        }
    }
    /// 插入peer 需要确认传输层协议

    #[inline]
    pub fn insert_peer(&self, cfg: PeerConfig) {
        let settings = self.settings.lock().unwrap();
        let mut index = self.peers.lock().unwrap();
        let mut secret = settings.secret.clone().with_peer(cfg.public_key);
        if let Some(psk) = cfg.preshared_key {
            secret.set_psk(psk);
        }
        let endpoint = cfg.endpoint.map(|addr| settings.inbound.endpoint_for(addr));
        index.insert(secret, cfg.allowed_ips, endpoint, cfg.persistent_keepalive);
    }
}


#[derive(Clone)]
pub struct DeviceControl
{
    inner: Arc<DeviceInner>,
    handle: Arc<AsyncMutex<DeviceHandle>>,
}