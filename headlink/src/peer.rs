use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use crate::client::ClientConnect;
use crate::db::entity::prelude::PeerModel;

#[derive(Clone)]
pub struct ModelInfo {}




#[derive(Clone,)]
pub struct OnlineInfo {
    pub connect: ClientConnect,
    pub port: u32,
    pub endpoint_addr: Option<String>,

    /// 扩展协议的接入端点
    pub extra_endpoints: HashMap<String, String>,
}

#[derive(Clone)]
pub struct VlinkPeer {
    pub pub_key: String,
    pub model: PeerModel,
    pub online_info: Option<OnlineInfo>,
    /*pub endpoint_addr: Option<IpAddr>,
    pub addr: Ipv4Addr,
    pub port: u32,
    pub online: bool,*/
}
impl VlinkPeer{
    pub fn is_online(&self) -> bool {
        self.online_info.is_some()
    }
}
impl From<PeerModel> for VlinkPeer {
    fn from(value: PeerModel) -> Self {
        Self {
            pub_key: value.pub_key.clone(),
            model: value,
            online_info: None,
        }
    }
}

