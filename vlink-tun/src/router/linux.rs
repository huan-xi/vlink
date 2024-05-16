use std::net::IpAddr;
use crate::router::IRouter;

pub struct Router{

}
impl Router{
    pub fn new(name:String)->Self {
        Self{

        }
    }
    pub fn add_route(&self, addr: IpAddr, mask: IpAddr) ->Result<(),crate::errors::Error> {
        Ok(())
    }
}
impl IRouter for Router{

}