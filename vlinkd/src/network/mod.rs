use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use log::{error, info, warn};
use tokio::sync::{Mutex, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::timeout;

use vlink_core::base64::decode_base64;
use vlink_core::secret::VlinkStaticSecret;
use vlink_tun::device::config::{ArgConfig, TransportConfig};
use vlink_tun::device::Device;
use vlink_tun::{InboundResult, Tun};

use crate::client::VlinkClient;
use crate::config::VlinkNetworkConfig;
use crate::handler;
use crate::handler::first_connected::request_for_config;
use crate::network::ctrl::NetworkCtrlCmd;
use crate::transport::nat_udp::{NatUdpTransport, NatUdpTransportParam};

pub mod ctrl;
pub mod types;

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
    device: Arc<RwLock<Option<Device>>>,
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
                device: Arc::new(Default::default()),
            }),
        }
    }
}

/// 协商peer与peer 之间的传输层协议
/// 怎么协商? 怎么确定断开，换协议
/// 服务端作为被动接受,

impl VlinkNetworkManagerInner {
    /// 启动网络管理器
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
                //文件中读取上一次缓存,
                todo!();
            }
        };
        if let Some(cfg) = config {
            self.start_device(cfg).await?;
        }


        //接受控制指令,操作device
        let client_c = self.client.clone();
        let device_c = self.device.clone();
        while let Some(cmd) = self.rx.lock().await.recv().await {
            info!("接受指令:{:?}",cmd);
            match cmd {
                NetworkCtrlCmd::ChangeIp => {
                    // device.change_ip();
                }
                NetworkCtrlCmd::PeerEnter(e) => {
                    //标记节点在线
                    // self.get_device().await?;
                    // info!("新增节点:ip:{},endpoint_addr:{:?}",e.ip,e.endpoint_addr);
                    // let c = bc_peer_enter2peer_config(&e)?;
                    let peer = device_c.read()
                        .await
                        .as_ref()
                        .ok_or(anyhow::anyhow!("device is none"))?
                        .get_peer_by_key(&decode_base64(e.pub_key.as_str())?.try_into().unwrap());
                    match peer {
                        None => {
                            warn!("peer not found");
                        }
                        Some(e) => {
                            *e.is_online.lock().unwrap() = true;
                        }
                    }
                    // device.insert_peer(c);
                }
                NetworkCtrlCmd::Connected => {
                    handler::connected::handler_connected(client_c.clone(), device_c.clone(), &args).await?;
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

    // async fn get_device(&self) -> anyhow::Result<dyn AsRef<Device>> {
    //     self.device.read().await.ok_or(anyhow::anyhow!("device is none"))
    // }
    async fn start_device(&self, config: VlinkNetworkConfig) -> anyhow::Result<()> {
        //启动peer 协议协商
        let trans = config.transports.clone();
        let device = Device::new(config.tun_name, config.device_config).await?;
        let tx = device.inbound_tx();

        self.device.write().await.replace(device);
        for cfg in trans {
            let txc = tx.clone();
            let cc = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = start_extra_transport(cc,txc, cfg).await {
                    error!("start extra transport error{e}")
                }
            });
        }
        Ok(())
    }
}

async fn start_extra_transport(cc: Arc<VlinkClient>, sender: Sender<InboundResult>, cfg: TransportConfig) -> anyhow::Result<()> {
    match cfg.proto.as_str() {
        "NatUdp" => {
            let param: NatUdpTransportParam = serde_json::from_str(&cfg.params)?;
            let mut ts = NatUdpTransport::new(cc,sender, param).await?;
            ts.start().await?;
        }
        _ => {

        }
    }
    Ok(())
}

impl Deref for VlinkNetworkManager {
    type Target = VlinkNetworkManagerInner;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}