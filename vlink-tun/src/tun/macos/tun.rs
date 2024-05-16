use std::io;
use std::mem::{size_of, size_of_val};
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use libc::{AF_INET, c_short, IFF_RUNNING, IFF_UP, IFNAMSIZ, SOCK_DGRAM};
use regex::Regex;
use tokio::io::unix::AsyncFd;
use log::debug;
use crate::tun::{Error, IFace};
use crate::Tun;
use crate::tun::unix::{SockAddr};
use super::sys;

#[inline]
fn parse_name(name: &str) -> Result<u32, Error> {
    if name == "utun" {
        return Ok(0);
    }
    let re = Regex::new(r"^utun([1-9]\d*|0)?$").unwrap();
    if !re.is_match(name) {
        return Err(Error::InvalidName);
    }
    name[4..]
        .parse()
        .map(|i: u32| i + 1)
        .map_err(|_| Error::InvalidName)
}

#[derive(Debug, Clone)]
pub struct NativeTun {
    fd: Arc<AsyncFd<OwnedFd>>,
    name: String,
    ctrl: Arc<AsyncFd<OwnedFd>>,
}

impl NativeTun {

    pub fn new(name: Option<String>) -> Result<Self, Error> {
        let idx = if let Some(name) = name {
            if name.len() > IFNAMSIZ {
                return Err(Error::IO(io::Error::new(io::ErrorKind::InvalidInput, "name too long")));
            }
            parse_name(name.as_ref())?
        } else {
            0u32
        };

        let fd = match unsafe {
            libc::socket(libc::PF_SYSTEM, libc::SOCK_DGRAM, libc::SYSPROTO_CONTROL)
        } {
            -1 => return Err(io::Error::last_os_error().into()),
            fd => unsafe { OwnedFd::from_raw_fd(fd) },
        };

        let info = libc::ctl_info {
            ctl_id: 0,
            ctl_name: {
                let mut buffer = [0; 96];
                for (i, o) in sys::UTUN_CONTROL_NAME.as_bytes().iter().zip(buffer.iter_mut()) {
                    *o = *i as _;
                }
                buffer
            },
        };
        if unsafe { libc::ioctl(fd.as_raw_fd(), libc::CTLIOCGINFO, &info) } < 0 {
            return Err(io::Error::last_os_error().into());
        }

        let addr = libc::sockaddr_ctl {
            sc_id: info.ctl_id,
            sc_len: size_of::<libc::sockaddr_ctl>() as _,
            sc_family: libc::AF_SYSTEM as _,
            ss_sysaddr: libc::AF_SYS_CONTROL as _,
            sc_unit: idx,
            sc_reserved: Default::default(),
        };
        if unsafe {
            libc::connect(
                fd.as_raw_fd(),
                &addr as *const libc::sockaddr_ctl as _,
                size_of_val(&addr) as _,
            )
        } < 0 {
            return Err(io::Error::last_os_error().into());
        }

        sys::set_nonblocking(fd.as_raw_fd())?;

        let name = unsafe { sys::get_iface_name(fd.as_raw_fd()) }?;

        let ctrl = match unsafe {
            libc::socket(AF_INET, SOCK_DGRAM, 0)
        } {
            -1 => return Err(io::Error::last_os_error().into()),
            fd => unsafe { OwnedFd::from_raw_fd(fd) },
        };


        let fd = Arc::new(AsyncFd::new(fd)?);
        let ctrl = Arc::new(AsyncFd::new(ctrl)?);
        Ok(Self { fd, name, ctrl })
    }
}

#[async_trait]
impl Tun for NativeTun {
    fn enabled(&self, value: bool) -> io::Result<()> {
        let mut req = sys::ifreq::new(self.name.as_ref());
        unsafe {
            sys::ioctl_get_flag(self.fd.as_raw_fd(), &mut req)?;
            if value {
                req.ifru.flags |= (IFF_UP | IFF_RUNNING) as c_short;
            } else {
                req.ifru.flags &= !(IFF_UP as c_short);
            }
            sys::ioctl_set_flag(self.fd.as_raw_fd(), &req)?;
            Ok(())
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn mtu(&self) -> Result<u16, Error> {
        let mut req = sys::ifreq::new(&self.name);

        unsafe { sys::ioctl_get_mtu(self.fd.as_raw_fd(), &mut req) }?;

        Ok(unsafe { req.ifru.mtu as _ })
    }

    fn set_mtu(&self, mtu: u16) -> Result<(), Error> {
        let mut req = sys::ifreq::new(&self.name);
        req.ifru.mtu = mtu as _;
        unsafe { sys::ioctl_set_mtu(self.fd.as_raw_fd(), &mut req) }?;
        Ok(())
    }


    fn address(&self) -> io::Result<Ipv4Addr> {
        let mut req = sys::ifreq::new(&self.name);
        unsafe {
            sys::ioctl_get_addr(self.ctrl.as_raw_fd(), &mut req)?;
            SockAddr::new(&req.ifru.addr).map(Into::into)
        }
    }

    fn set_address(&self, value: Ipv4Addr) -> io::Result<()> {
        let mut req = sys::ifreq::new(&self.name);
        req.ifru.addr = SockAddr::from(value).into();
        unsafe { sys::ioctl_set_addr(self.ctrl.as_raw_fd(), &req) }?;
        Ok(())
    }
    fn netmask(&self) -> io::Result<Ipv4Addr> {
        let mut req = sys::ifreq::new(&self.name);
        unsafe {
            sys::ioctl_get_netmask(self.ctrl.as_raw_fd(), &mut req)?;
            SockAddr::new(&req.ifru.addr).map(Into::into)
        }
    }

    fn set_netmask(&self, value: Ipv4Addr) -> io::Result<()> {
        let mut req = sys::ifreq::new(&self.name);
        req.ifru.addr = SockAddr::from(value).into();
        unsafe { sys::ioctl_set_netmask(self.ctrl.as_raw_fd(), &req) }?;
        Ok(())
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
                Ok(Ok(n)) if n >= 4 => {
                    debug!("TUN read {} bytes", n);
                    buf.advance(4);
                    buf.truncate(n - 4);
                    return Ok(buf.freeze().to_vec());
                }
                Ok(Err(e)) => return Err(e.into()),
                _ => continue,
            }
        }
    }

    async fn send(&self, buf: &[u8]) -> Result<(), Error> {
        let buf = {
            let mut m = vec![0u8; 4 + buf.len()];
            m[3] = match buf[0] >> 4 {
                4 => 0x2,
                6 => 0x1e,
                _ => return Err(Error::InvalidIpPacket),
            };
            m[4..].copy_from_slice(buf);
            m
        };

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


#[cfg(test)]
mod tests {
    use libc::sleep;
    use super::*;


    #[test]
    fn test_parse_name() {
        let success_cases = [("utun", 0), ("utun0", 1), ("utun42", 43)];

        for (input, expected) in success_cases {
            let rv = parse_name(input);
            assert!(rv.is_ok());
            assert_eq!(rv.unwrap(), expected);
        }

        let failure_cases = ["utun04", "utun007", "utun42foo", "utunfoo", "futun"];

        for input in failure_cases {
            assert!(parse_name(input).is_err())
        }
    }
}
