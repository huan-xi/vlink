use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use async_trait::async_trait;
use crate::Tun;
use crate::tun::{Error};
use crate::tun::windows::{tap, tun};

#[derive(Clone)]
pub enum NativeTun {
    Tap(tap::Device),
    Tun(Arc<tun::Device>),
}

impl NativeTun {
    pub fn new(name: Option<String>, tap: bool) ->  Result<Self, crate::errors::Error>  {
        let name = name.unwrap_or_else(|| "vlink-tun".to_string());
        if tap {
            Ok(NativeTun::Tap(tap::Device::new(name)?))
        } else {
            Ok(NativeTun::Tun(Arc::new(tun::Device::new(name)?)))
        }
    }
}

#[async_trait]
impl Tun for NativeTun {
    fn enabled(&self, value: bool) -> std::io::Result<()> {
        match self {
            NativeTun::Tap(t) => t.enabled(value),
            NativeTun::Tun(t) => t.enabled(value)
        }
    }

    fn name(&self) -> &str {
        match self {
            NativeTun::Tap(t) => t.name(),
            NativeTun::Tun(t) => t.name()
        }
    }

    fn mtu(&self) -> Result<u16, Error> {
        match self {
            NativeTun::Tap(t) => t.mtu(),
            NativeTun::Tun(t) => t.mtu()
        }
    }

    fn set_mtu(&self, mtu: u16) -> Result<(), Error> {
        match self {
            NativeTun::Tap(t) => t.set_mtu(mtu),
            NativeTun::Tun(t) => t.set_mtu(mtu)
        }
    }

    fn address(&self) -> std::io::Result<Ipv4Addr> {
        match self {
            NativeTun::Tap(t) => t.address(),
            NativeTun::Tun(t) => t.address()
        }
    }

    fn set_address(&self, value: Ipv4Addr) -> std::io::Result<()> {
        match self {
            NativeTun::Tap(t) => t.set_address(value),
            NativeTun::Tun(t) => t.set_address(value)
        }
    }

    fn netmask(&self) -> std::io::Result<Ipv4Addr> {
        match self {
            NativeTun::Tap(t) => t.netmask(),
            NativeTun::Tun(t) => t.netmask()
        }
    }

    fn set_netmask(&self, value: Ipv4Addr) -> std::io::Result<()> {
        match self {
            NativeTun::Tap(t) => t.set_netmask(value),
            NativeTun::Tun(t) => t.set_netmask(value)
        }
    }

    async fn recv(&self) -> Result<Vec<u8>, Error> {
        match self {
            NativeTun::Tap(t) => t.recv().await,
            NativeTun::Tun(t) => t.recv().await
        }
    }

    async fn send(&self, buf: &[u8]) -> Result<(), Error> {
        match self {
            NativeTun::Tap(t) => t.send(buf).await,
            NativeTun::Tun(t) => t.send(buf).await
        }
    }
    fn set_ip(&self, address: Ipv4Addr, mask: Ipv4Addr) -> io::Result<()> {
        match self {
            NativeTun::Tap(t) => t.set_ip(address, mask),
            NativeTun::Tun(t) => t.set_ip(address, mask)
        }
    }
}