use std::collections::HashSet;
use std::net::{SocketAddr, SocketAddrV4};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use base64::Engine;
use log::{error, info};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_core::proto::pb::abi::PeerEnter;
use vlink_tun::device::Device;
use vlink_tun::{LocalStaticSecret, PeerConfig, Tun};
use crate::client::VlinkClient;
use vlink_core::proto::pb::abi::*;
use vlink_tun::device::config::{ArgConfig, TransportConfig, TransportType};
use vlink_tun::device::peer::cidr::Cidr;
use crate::config::{bc_peer_enter2peer_config, VlinkNetworkConfig};
use crate::transport::nat_udp::{NatUdpTransport, NatUdpTransportParam};
use crate::utils::iface::find_my_ip;

pub enum NetworkStatus {
    Running,
    Stop,
}

#[derive(Debug)]
pub enum NetworkCtrlCmd {
    ChangeIp,
    PeerEnter(BcPeerEnter),
    /// 重新加入网络
    Reenter,
}

#[derive(Clone)]
pub struct NetworkCtrl {
    pub sender: mpsc::Sender<NetworkCtrlCmd>,
}

impl Deref for NetworkCtrl {
    type Target = mpsc::Sender<NetworkCtrlCmd>;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}


#[derive(Clone)]
pub struct VlinkNetworkManager {
    inner: Arc<VlinkNetworkManagerInner>,
}

pub struct VlinkNetworkManagerInner {
    config: RwLock<Option<VlinkNetworkConfig>>,
    client: VlinkClient,
    // status: RwLock<NetworkStatus>,
}

/// change_ip
/// add_peer
impl VlinkNetworkManager {
    pub fn new(client: VlinkClient, config: VlinkNetworkConfig) -> Self {
        Self {
            inner: Arc::new(VlinkNetworkManagerInner {
                config: RwLock::new(Some(config)),
                client,
            }),
        }
    }
}

/// 协商peer与peer 之间的传输层协议
/// 怎么协商? 怎么确定断开，换协议
/// 服务端作为被动接受,

impl VlinkNetworkManagerInner {
    /// 阻塞


    pub async fn start(&self, mut rx: Receiver<NetworkCtrlCmd>) -> anyhow::Result<()> {
        let config = self.config.write().unwrap().take().unwrap();
        //todo 检查网段冲突
        let device = Device::new(config.tun_name, config.device_config).await?;
        // 启动额外传输层
        start_transports(&config.transports).await?;

        info!("启动接口:{}",device.tun.name());
        //上报设备信息
        send_enter(&self.client, &device, &config.arg_config).await?;
        let event_loop = {};
        //接受控制指令,操作device
        while let Some(cmd) = rx.recv().await {
            info!("接受指令:{:?}",cmd);
            match cmd {
                NetworkCtrlCmd::ChangeIp => {
                    // device.change_ip();
                }
                NetworkCtrlCmd::PeerEnter(e) => {
                    info!("新增节点:ip:{},endpoint_addr:{:?}",e.ip,e.endpoint_addr);
                    let c = bc_peer_enter2peer_config(&e)?;
                    device.insert_peer(c);
                }
                NetworkCtrlCmd::Reenter => {
                    send_enter(&self.client, &device, &config.arg_config).await?;
                }
            }
        }

        Ok(())
    }
}


async fn start_transports(trans: &Vec<TransportConfig>) -> anyhow::Result<()>{
    for cfg in trans.iter() {
        match cfg.trans_type{
            TransportType::NatUdp => {
                let param: NatUdpTransportParam = serde_json::from_str(&cfg.params)?;
                NatUdpTransport::new(param).await?;
            }
        }
    }
    Ok(())
}


/// 进入网络信息
/// 本机ip+udp port
pub async fn send_enter(client: &VlinkClient, device: &Device, arg: &ArgConfig) -> anyhow::Result<()> {
    client.send(ToServerData::PeerEnter(PeerEnter {
        ip: device.tun_addr.to_string(),
        endpoint_addr: arg.endpoint_addr.clone(),
        port: device.port as u32,
    })).await?;
    Ok(())
}


impl Deref for VlinkNetworkManager {
    type Target = VlinkNetworkManagerInner;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}