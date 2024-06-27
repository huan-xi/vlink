use std::str::FromStr;
use std::sync::Arc;
use anyhow::anyhow;
use tokio::sync::mpsc::Sender;
use vlink_tun::device::config::TransportConfig;
use vlink_tun::device::event::DevicePublisher;
use vlink_tun::InboundResult;
use crate::client::VlinkClient;
use crate::network::ExtraProto;
use crate::transport::proto::dynamic_ip::DipParam;
use crate::transport::proto::nat_tcp::{NatTcpTransport, NatTcpTransportParam};
use crate::transport::proto::nat_udp::{NatUdpTransport, NatUdpTransportParam};

pub(crate) async fn start_extra_transport(cc: Arc<VlinkClient>,
                               sender: Sender<InboundResult>,
                               cfg: TransportConfig, event_pub: DevicePublisher) -> anyhow::Result<()> {

    let proto = ExtraProto::from_str(cfg.proto.as_str()).map_err(|_| anyhow!("扩展协议:{}不支持",cfg.proto))?;
    match proto {
        ExtraProto::NatUdp => {
            let param: NatUdpTransportParam = serde_json::from_str(&cfg.params)?;
            let mut ts = NatUdpTransport::new(sender, param, event_pub).await?;
            ts.start().await?;
        }
        ExtraProto::NatTcp => {
            let param: NatTcpTransportParam = serde_json::from_str(&cfg.params)?;
            let mut ts = NatTcpTransport::new(cc, sender, param, event_pub).await?;
            ts.start().await?;
        }
        ExtraProto::Dip => {
            let param: DipParam = serde_json::from_str(&cfg.params)?;

        }

        _ => {}
    }
    Ok(())
}