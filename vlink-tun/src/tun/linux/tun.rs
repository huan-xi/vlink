use std::io;
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::BytesMut;
use libc::{__c_anonymous_ifr_ifru, IFF_NO_PI, IFF_TUN, IFF_VNET_HDR};
use nix::fcntl::{self, OFlag};
use nix::sys::stat::Mode;
use tokio::io::unix::AsyncFd;
use log::debug;

use crate::tun::linux::sys::{self, get_mtu, ioctl_tun_set_iff, set_mtu, set_nonblocking};
use crate::tun::{Error, IFace};
use crate::Tun;

const DEVICE_PATH: &str = "/dev/net/tun";

#[derive(Clone)]
pub struct NativeTun {
    fd: Arc<AsyncFd<OwnedFd>>,
    ctrl: Arc<AsyncFd<OwnedFd>>,
    name: String,
}

impl NativeTun {
    pub fn new(name: Option<String>) -> Result<Self, Error> {
        if name.len() > 16 {
            return Err(Error::InvalidName);
        }
        let fd = fcntl::open(DEVICE_PATH, OFlag::O_RDWR | OFlag::O_CLOEXEC, Mode::empty())
            .map(|fd| unsafe { OwnedFd::from_raw_fd(fd) })
            .map_err(Error::Sys)?;

        let mut ifr = sys::new_ifreq(name);
        ifr.ifr_ifru = __c_anonymous_ifr_ifru {
            ifru_flags: (IFF_TUN | IFF_NO_PI) as _,
        };
        let _ = IFF_VNET_HDR; // TODO: enable

        unsafe { ioctl_tun_set_iff(fd.as_raw_fd(), &ifr) }?;
        set_nonblocking(fd.as_raw_fd())?;
        //= Fd::new(libc::socket(AF_INET, SOCK_DGRAM, 0))?;
        let ctrl = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
        if ctrl < 0 {
            return Err(Error::IO(io::Error::last_os_error()));
        };

        Ok(Self {
            fd: Arc::new(AsyncFd::new(fd)?),
            ctrl: Arc::new(AsyncFd::new(unsafe { OwnedFd::from_raw_fd(ctrl) })?),
            name: name.to_owned(),
        })
    }
}

#[async_trait]
impl Tun for NativeTun {
    // fn enabled(&self, value: bool) -> io::Result<()> {
    //     todo!()
    // }

    fn name(&self) -> &str {
        &self.name
    }

    fn mtu(&self) -> Result<u16, Error> {
        get_mtu(&self.name)
    }

    fn set_mtu(&self, mtu: u16) -> Result<(), Error> {
        set_mtu(&self.name, mtu)
    }

    fn address(&self) -> io::Result<Ipv4Addr> {
        //sys::siocgifaddr(self.fd.as_raw_fd())
        todo!();
    }

    fn set_address(&self, value: Ipv4Addr) -> io::Result<()> {
        todo!()
    }

    fn netmask(&self) -> io::Result<Ipv4Addr> {
        todo!()
    }

    fn set_netmask(&self, value: Ipv4Addr) -> io::Result<()> {
        todo!()
    }

    async fn recv(&self) -> Result<Vec<u8>, Error> {
        let mut buf = BytesMut::zeroed(1500);

        loop {
            let ret = {
                let mut guard = self.fd.readable().await?;
                guard.try_io(|inner| unsafe {
                    let ret = libc::read(inner.as_raw_fd(), buf.as_mut_ptr() as _, buf.len());
                    if ret < 0 {
                        Err::<usize, io::Error>(io::Error::last_os_error())
                    } else {
                        Ok(ret as usize)
                    }
                })
            };

            match ret {
                Ok(Ok(n)) => {
                    debug!("TUN read {} bytes", n);
                    buf.truncate(n);
                    return Ok(buf.freeze().to_vec());
                }
                Ok(Err(e)) => return Err(e.into()),
                _ => continue,
            }
        }
    }

    async fn send(&self, buf: &[u8]) -> Result<(), Error> {
        let mut guard = self.fd.writable().await?;
        let ret = guard.try_io(|inner| unsafe {
            let ret = libc::write(inner.as_raw_fd(), buf.as_ptr() as _, buf.len());
            if ret < 0 {
                Err::<usize, io::Error>(io::Error::last_os_error())
            } else {
                Ok(ret as usize)
            }
        });

        match ret {
            Ok(Ok(_)) => return Ok(()),
            Ok(Err(e)) => return Err(e.into()),
            _ => {}
        }

        Ok(())
    }
}

impl IFace for NativeTun {
    fn set_ip(&self, address: Ipv4Addr, mask: Ipv4Addr) -> io::Result<()> {
        self.set_address(address)?;
        self.set_netmask(mask)
    }
}