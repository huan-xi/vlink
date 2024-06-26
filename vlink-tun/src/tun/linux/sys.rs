use std::mem;
use std::os::fd::{AsRawFd, RawFd};

use libc::{__c_anonymous_ifr_ifru, c_char, ifreq};
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::socket::{socket, AddressFamily, SockFlag, SockType};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad};

use crate::tun::Error;

ioctl_write_ptr_bad!(ioctl_tun_set_iff, 0x400454ca, ifreq);
ioctl_read_bad!(ioctl_tun_get_iff, 0x800454d2, ifreq);
ioctl_write_ptr_bad!(ioctl_set_mtu, 0x8922, ifreq);
ioctl_read_bad!(ioctl_get_mtu, 0x8921, ifreq);

ioctl_read_bad!(siocgifaddr,0x8915,ifreq);
ioctl_write_ptr_bad!(siocsifaddr,0x8916, ifreq);

ioctl_read_bad!(siocgifnetmask,0x891b,ifreq);
ioctl_write_ptr_bad!(siocsifnetmask,0x891c,ifreq);

ioctl_read_bad!(siocgifflags , 0x8913, ifreq);
ioctl_write_ptr_bad!( siocsifflags , 0x8914, ifreq);
pub fn new_ifreq(name: &str) -> ifreq {
    let mut ifr: ifreq = unsafe { mem::zeroed() };
    let ifr_name: Vec<c_char> = name.as_bytes().iter().map(|c| *c as _).collect();
    ifr.ifr_name[..name.len()].copy_from_slice(&ifr_name);
    ifr
}

pub fn set_nonblocking(fd: RawFd) -> Result<(), Error> {
    let flag = fcntl(fd, FcntlArg::F_GETFL)
        .map(OFlag::from_bits_retain)
        .map_err(Error::Sys)?;
    let flag = OFlag::O_NONBLOCK | flag;
    fcntl(fd, FcntlArg::F_SETFL(flag)).map_err(Error::Sys)?;
    Ok(())
}

pub fn set_mtu(name: &str, mtu: u16) -> Result<(), Error> {
    let fd = socket(
        AddressFamily::Inet,
        SockType::Datagram,
        SockFlag::empty(),
        None,
    )
        .map_err(Error::Sys)?;
    let mut ifr = new_ifreq(name);
    ifr.ifr_ifru = __c_anonymous_ifr_ifru { ifru_mtu: mtu as _ };
    unsafe { ioctl_set_mtu(fd.as_raw_fd(), &ifr) }.map_err(Error::Sys)?;
    Ok(())
}

pub fn get_mtu(name: &str) -> Result<u16, Error> {
    let fd = socket(
        AddressFamily::Inet,
        SockType::Datagram,
        SockFlag::empty(),
        None,
    )
        .map_err(Error::Sys)?;
    let mut ifr = new_ifreq(name);
    unsafe { ioctl_get_mtu(fd.as_raw_fd(), &mut ifr) }.map_err(Error::Sys)?;
    Ok(unsafe { ifr.ifr_ifru.ifru_mtu as _ })
}
