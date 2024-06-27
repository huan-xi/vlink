use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use vlink_tun::device::event::DevicePublisher;
use vlink_tun::InboundResult;
use crate::client::VlinkClient;
use crate::transport::proto::nat_tcp::NatTcpTransportParam;

/// Websocket transport 可以被nginx 代理


pub const PROTO_NAME: &str = "Ws";
pub const DEFAULT_PORT: u16 = 8521;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebsocketTransportParam {
    /// 端口默认8521
    port: Option<u16>,
    /// 监听地址默认0.0.0.0
    listen: Option<String>,
}


pub struct WebsocketTransport {}

impl WebsocketTransport {
    pub async fn spawn(client: Arc<VlinkClient>,
                       sender: Sender<InboundResult>,
                       param: NatTcpTransportParam, event_pub: DevicePublisher) -> anyhow::Result<Self> {
        //启动websocket服务

        return Ok(Self {

        });
    }
}

pub struct WebsocketTransportClient {}