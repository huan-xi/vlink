use std::net::IpAddr;
use std::process::Command;
use log::info;
use crate::device::peer::cidr::Cidr;
use crate::router::IRouter;

pub struct Router {
    tun_name: String,
}

impl Router {
    pub fn new(tun_name: String) -> Self {
        Router { tun_name }
    }
    pub fn add_route(&self, addr: IpAddr, mask: IpAddr) ->Result<(),crate::errors::Error> {
        let route_add_str: String = format!(
            "route -n add {} -netmask {} -interface {}",
            addr, mask, self.tun_name
        );
        info!("route_add_str:{}", route_add_str);
        let route_add_out = Command::new("sh")
            .arg("-c")
            .arg(&route_add_str)
            .output()
            .expect("sh exec error!");
        let str = route_add_out.stdout.to_vec();
        info!("route_add_out:{}", String::from_utf8_lossy(str.as_slice()));
        Ok(())
    }
}

impl IRouter for Router {}