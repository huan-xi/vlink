pub mod ctrl;
pub mod types;

use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use log::info;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::Receiver;
use tokio::time::timeout;

use vlink_core::proto::pb::abi::*;
use vlink_core::proto::pb::abi::PeerEnter;
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_core::secret::VlinkStaticSecret;
use vlink_tun::device::config::{ArgConfig, TransportConfig, TransportType};
use vlink_tun::device::Device;
use vlink_tun::Tun;

use crate::client::VlinkClient;
use crate::config::VlinkNetworkConfig;
use crate::handler::first_connected::request_for_config;
use crate::network::ctrl::NetworkCtrlCmd;
use crate::transport::nat_udp::{NatUdpTransport, NatUdpTransportParam};

pub enum NetworkStatus {
    Running,
    Stop,
}



#[derive(Clone)]
pub struct VlinkNetworkManager {
    inner: Arc<VlinkNetworkManagerInner>,
}


pub struct VlinkNetworkManagerInner {
    client: Arc<VlinkClient>,
    rx: Arc<Mutex<Receiver<NetworkCtrlCmd>>>,
    secret: VlinkStaticSecret,
    // status: RwLock<NetworkStatus>,
}

/// change_ip
/// add_peer
impl VlinkNetworkManager {
    pub fn new(client: VlinkClient, rx: Receiver<NetworkCtrlCmd>, secret: VlinkStaticSecret) -> Self {
        Self {
            inner: Arc::new(VlinkNetworkManagerInner {
                client: Arc::new(client),
                rx: Arc::new(Mutex::new(rx)),
                secret,
            }),
        }
    }
}

/// 协商peer与peer 之间的传输层协议
/// 怎么协商? 怎么确定断开，换协议
/// 服务端作为被动接受,

impl VlinkNetworkManagerInner {
    /// 阻塞


    pub async fn start(&self, args: ArgConfig) -> anyhow::Result<()> {
        //启动设备,本地文件读取配置信息,等待首次连接配置
        let rxc = self.rx.clone();
        let client_c = self.client.clone();
        let secret_c = self.secret.clone();
        let config = match timeout(Duration::from_secs(2), async move {
            if let Some(NetworkCtrlCmd::FirstConnected) = rxc.lock().await.recv().await {
                info!("首次连接");
                return true;
            }
            false
        }).await {
            Ok(_) => {
                //向服务器请求配置并保存
                Some(request_for_config(client_c, *secret_c.private_key.as_bytes(), &args).await?)
            }
            Err(_) => {
                //文件中读取配置,
                todo!();
            }
        };
        if let Some(cfg) = config {
            start_device(cfg).await?;
        }


        //接受控制指令,操作device
        while let Some(cmd) = self.rx.lock().await.recv().await {
            info!("接受指令:{:?}",cmd);
            match cmd {
                NetworkCtrlCmd::ChangeIp => {
                    // device.change_ip();
                }
                NetworkCtrlCmd::PeerEnter(e) => {
                    info!("新增节点:ip:{},endpoint_addr:{:?}",e.ip,e.endpoint_addr);
                    // let c = bc_peer_enter2peer_config(&e)?;
                    // device.insert_peer(c);
                }
                NetworkCtrlCmd::Connected => {

                    // send_enter(&self.client, &device, &config.arg_config).await?;
                }
                NetworkCtrlCmd::FirstConnected => {
                    // 请求配置并保存
                    // request_for_config(&self.client).await?;
                    //new_device()
                }
                _ => {}
            }
        }

        Ok(())


        /* let config = self.config.write().unwrap().take().unwrap();
         //todo 检查网段冲突

         // 启动额外传输层
         start_transports(&config.transports).await?;

         info!("启动接口:{}",device.tun.name());
         //上报设备信息
         send_enter(&self.client, &device, &config.arg_config).await?;
         let event_loop = {};
        */
    }
}

async fn start_device(config: VlinkNetworkConfig) -> anyhow::Result<()> {
    let device = Device::new(config.tun_name, config.device_config).await?;

    Ok(())
}


async fn start_transports(trans: &Vec<TransportConfig>) -> anyhow::Result<()> {
    for cfg in trans.iter() {
        match cfg.trans_type {
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