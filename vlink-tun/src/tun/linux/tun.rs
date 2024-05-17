use std::{io, mem, ptr};
use std::ffi::CStr;
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::BytesMut;
use libc::{__c_anonymous_ifr_ifru, c_char, IFF_NO_PI, IFF_TUN, IFF_VNET_HDR, ifreq};
use nix::fcntl::{self, OFlag};
use nix::sys::stat::Mode;
use tokio::io::unix::AsyncFd;
use log::debug;

use crate::tun::linux::sys::{self, get_mtu, ioctl_tun_set_iff, set_mtu, set_nonblocking};
use crate::tun::{Error, IFace};
use crate::Tun;
use crate::tun::unix::SockAddr;

const DEVICE_PATH: &str = "/dev/net/tun";

#[derive(Clone)]
pub struct NativeTun {
    fd: Arc<AsyncFd<OwnedFd>>,
    ctrl: Arc<AsyncFd<OwnedFd>>,
    name: String,
}

impl NativeTun {
    pub fn new(name: Option<String>) -> Result<Self, Error> {
        if let Some(n) = &name {
            if n.len() > 16 {
                return Err(Error::InvalidName);
            }
        };

        let fd = fcntl::open(DEVICE_PATH, OFlag::O_RDWR | OFlag::O_CLOEXEC, Mode::empty())
            .map(|fd| unsafe { OwnedFd::from_raw_fd(fd) })
            .map_err(Error::Sys)?;

        let mut ifr = match &name {
            None => {
                unsafe { mem::zeroed() }
            }
            Some(name) => {
                sys::new_ifreq(name.as_str())
            }
        };
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
        let name = unsafe {
            CStr::from_ptr(ifr.ifr_name.as_ptr())
                .to_string_lossy()
                .to_string()
        };
        Ok(Self {
            fd: Arc::new(AsyncFd::new(fd)?),
            ctrl: Arc::new(AsyncFd::new(unsafe { OwnedFd::from_raw_fd(ctrl) })?),
            name,
        })
    }
    unsafe fn request(&self) -> ifreq {
        let mut req: ifreq = mem::zeroed();
        ptr::copy_nonoverlapping(
            self.name.as_ptr() as *const c_char,
            req.ifr_name.as_mut_ptr(),
            self.name.len(),
        );
        req
    }
}

#[async_trait]
impl Tun for NativeTun {
    fn enabled(&self, value: bool) -> io::Result<()> {
        unsafe {
            let mut req = self.request();

            sys::siocgifflags(self.ctrl.as_raw_fd(), &mut req)?;
            if value {
                req.ifr_ifru.ifru_flags |= (libc::IFF_UP | libc::IFF_RUNNING) as libc::c_short;
            } else {
                req.ifr_ifru.ifru_flags &= !(libc::IFF_UP as libc::c_short);
            }
            sys::siocsifflags(self.ctrl.as_raw_fd(), &req)?;

            Ok(())
        }
    }

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
        unsafe {
            let mut req = self.request();
            sys::siocgifaddr(self.ctrl.as_raw_fd(), &mut req)?;
            SockAddr::new(&req.ifr_ifru.ifru_addr).map(Into::into)
        }
    }

    fn set_address(&self, value: Ipv4Addr) -> io::Result<()> {
        unsafe {
            let mut req = self.request();
            req.ifr_ifru.ifru_addr = SockAddr::from(value).into();
            sys::siocsifaddr(self.ctrl.as_raw_fd(), &req)?;
            Ok(())
        }
    }

    fn netmask(&self) -> io::Result<Ipv4Addr> {
        unsafe {
            let mut req = self.request();
            sys::siocgifnetmask(self.ctrl.as_raw_fd(), &mut req)?;
            SockAddr::new(&req.ifr_ifru.ifru_netmask).map(Into::into)
        }
    }

    fn set_netmask(&self, value: Ipv4Addr) -> io::Result<()> {
        unsafe {
            let mut req = self.request();
            req.ifr_ifru.ifru_netmask = SockAddr::from(value).into();
            sys::siocsifnetmask(self.ctrl.as_raw_fd(), &req)?;
            Ok(())
        }
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