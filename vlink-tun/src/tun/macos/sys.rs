use std::{io, mem, ptr};
use std::ffi::CStr;
use std::os::fd::RawFd;

use libc::*;
use nix::{ioctl_readwrite, ioctl_write_ptr};
use nix::fcntl::{fcntl, FcntlArg, OFlag};

use crate::tun::Error;

pub const SIOCSIFMTU: u64 = 0x80206934;
pub const SIOCGIFMTU: u64 = 0xc0206933;

ioctl_readwrite!(ioctl_get_mtu, 'i', 51, ifreq);
ioctl_write_ptr!(ioctl_set_mtu,'i', 52, ifreq);


ioctl_write_ptr!(ioctl_set_addr,'i', 12, ifreq);
ioctl_readwrite!(ioctl_get_addr, 'i', 33,ifreq);

ioctl_write_ptr!(ioctl_set_netmask,'i', 22, ifreq);
ioctl_readwrite!(ioctl_get_netmask, 'i', 37,ifreq);

//设置flag
ioctl_write_ptr!( ioctl_set_flag , 'i', 16,ifreq);
ioctl_readwrite!( ioctl_get_flag , 'i', 17, ifreq);

// ioctl!(write siocsifaddr with 'i', 12; ifreq);
// ioctl!(readwrite siocgifaddr with 'i', 33; ifreq);

// ioctl!(write siocsifbrdaddr with 'i', 19; ifreq);
// ioctl!(readwrite siocgifbrdaddr with 'i', 35; ifreq);
//
// ioctl!(write siocsifnetmask with 'i', 22; ifreq);
// ioctl!(readwrite siocgifnetmask with 'i', 37; ifreq);
pub const UTUN_CONTROL_NAME: &str = "com.apple.net.utun_control";

#[repr(C)]
#[derive(Copy, Clone)]
pub union ifrn {
    pub name: [c_char; IFNAMSIZ],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ifdevmtu {
    pub current: c_int,
    pub min: c_int,
    pub max: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ifku {
    pub ptr: *mut c_void,
    pub value: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ifru {
    pub addr: sockaddr,
    pub dstaddr: sockaddr,
    pub broadaddr: sockaddr,

    pub flags: c_short,
    pub metric: c_int,
    pub mtu: c_int,
    pub phys: c_int,
    pub media: c_int,
    pub intval: c_int,
    pub data: *mut c_void,
    pub devmtu: ifdevmtu,
    pub wake_flags: c_uint,
    pub route_refcnt: c_uint,
    pub cap: [c_int; 2],
    pub functional_type: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ifreq {
    pub ifrn: ifrn,
    pub ifru: ifru,
}

impl ifreq {
    pub fn new(name: &str) -> Self {
        let mut me: Self = unsafe { mem::zeroed() };
        unsafe {
            ptr::copy_nonoverlapping(
                name.as_ptr() as *const c_char,
                me.ifrn.name.as_mut_ptr(),
                name.len(),
            )
        }
        me
    }
}

pub fn set_nonblocking(fd: RawFd) -> Result<(), Error> {
    let flag = fcntl(fd, FcntlArg::F_GETFL)
        .map(OFlag::from_bits_retain)
        .map_err(Error::Sys)?;
    let flag = OFlag::O_NONBLOCK | flag;
    fcntl(fd, FcntlArg::F_SETFL(flag)).map_err(Error::Sys)?;
    Ok(())
}

pub unsafe fn get_iface_name(fd: RawFd) -> Result<String, io::Error> {
    const MAX_LEN: usize = 256;
    let mut name = [0u8; MAX_LEN];
    let mut name_len: libc::socklen_t = name.len() as _;
    if libc::getsockopt(
        fd,
        libc::SYSPROTO_CONTROL,
        libc::UTUN_OPT_IFNAME,
        name.as_mut_ptr() as _,
        &mut name_len,
    ) < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(CStr::from_ptr(name.as_ptr() as *const libc::c_char)
        .to_string_lossy()
        .into())
}

#[cfg(test)]
mod tests {}
