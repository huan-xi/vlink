mod error;

use std::io;
use std::net::Ipv4Addr;
pub use error::Error;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::NativeTun;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::NativeTun;

#[cfg(unix)]
mod unix;

use async_trait::async_trait;

#[async_trait]
pub trait Tun: Send + Sync + Clone {
    // fn enabled(&self, value: bool) -> io::Result<()>;

    fn name(&self) -> &str;
    fn mtu(&self) -> Result<u16, Error>;
    fn set_mtu(&self, mtu: u16) -> Result<(), Error>;
    fn address(&self) -> io::Result<Ipv4Addr>;
    fn set_address(&self, value: Ipv4Addr) -> io::Result<()>;
    fn netmask(&self) -> io::Result<Ipv4Addr>;
    fn set_netmask(&self, value: Ipv4Addr) -> io::Result<()>;

    async fn recv(&self) -> Result<Vec<u8>, Error>;
    async fn send(&self, buf: &[u8]) -> Result<(), Error>;
}

pub trait IFace {
    fn set_ip(&self, address: Ipv4Addr, mask: Ipv4Addr) -> std::io::Result<()>;
}