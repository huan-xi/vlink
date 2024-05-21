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
use crate::network::config::VlinkNetworkConfig;
use vlink_core::proto::pb::abi::*;
use vlink_tun::device::config::ArgConfig;
use vlink_tun::device::peer::cidr::Cidr;
use crate::utils::iface::find_my_ip;

pub mod config;

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

impl VlinkNetworkManagerInner {
    /// 阻塞
    pub async fn start(&self, mut rx: Receiver<NetworkCtrlCmd>) -> anyhow::Result<()> {
        let config = self.config.write().unwrap().take().unwrap();
        //todo 检查网段冲突
        let device = Device::new(config.tun_name, config.device_config).await?;
        info!("启动接口:{}",device.tun.name());
        //上报设备信息
        send_enter(&self.client, &device, &config.arg_config).await?;

        //接受控制指令,操作device
        while let Some(cmd) = rx.recv().await {
            info!("接受指令:{:?}",cmd);
            match cmd {
                NetworkCtrlCmd::ChangeIp => {
                    // device.change_ip();
                }
                NetworkCtrlCmd::PeerEnter(e) => {
                    info!("新增节点:{:?}",e);
                    let pk = crate::base64Encoding.decode(e.pub_key.as_str())?;
                    let public_key = pk.as_slice().try_into()?;
                    let cidr = Cidr::new(e.ip.as_str().parse().unwrap(), 32);
                    let allowed_ips = HashSet::from([cidr]);
                    info!("addr:{:?}", e.endpoint_addr);
                    device.insert_peer(PeerConfig {
                        public_key,
                        allowed_ips,
                        endpoint: match e.endpoint_addr {
                            None => {None}
                            Some(addr) => {
                                //Some(SocketAddr::V4(SocketAddrV4::new(e.endpoint_addr.parse().unwrap(), e.port as u16)))
                                Some(SocketAddr::new(addr.parse()?, e.port as u16))
                            }
                        },
                        preshared_key: None,
                        lazy: false,
                        no_encrypt: false,
                        persistent_keepalive: None,
                    });
                    // device.add_peer();
                }
                NetworkCtrlCmd::Reenter => {
                    send_enter(&self.client, &device,&config.arg_config).await?;
                }
            }
        }

        Ok(())
    }
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