use serde::{Deserialize, Serialize};

/// 仿ddns 动态公网ip
/// 连接服务器成功后上报注册公网ip
///
pub const PROTO_NAME: &str = "Dip";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DipParam {
    /// 外部端口
    pub_port: u16,
}

pub struct DynamicIpTransport {}