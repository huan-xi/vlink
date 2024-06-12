use serde::{Deserialize, Serialize};

/// ddns 协议,可以基于其他协议传输

pub const PROTO_NAME: &str = "Ddns";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DdnsTransportParam {
    /// 外部端口
    pub_port: u16,
}

pub struct DdnsTransport {}

impl DdnsTransport {
    pub fn start() {
        //启动ddns 服务,注册当前公网ip,stun 服务器获取公网

    }
}