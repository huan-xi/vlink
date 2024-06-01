use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use base64::Engine;
use log::{debug, error, info, warn};
use strum::{AsRefStr, EnumString};
use tokio::sync::{Mutex, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::timeout;

use vlink_core::base64::decode_base64;
use vlink_core::rw_map::RwMap;
use vlink_core::secret::VlinkStaticSecret;
use vlink_tun::{InboundResult, Tun};
use vlink_tun::device::config::{ArgConfig, TransportConfig};
use vlink_tun::device::Device;
use vlink_tun::device::event::DevicePublisher;

use crate::client::VlinkClient;
use crate::config::VlinkNetworkConfig;
use crate::handler;
use crate::handler::first_connected::request_for_config;
use crate::network::ctrl::NetworkCtrlCmd;
use crate::transport::ext_transport_selector::ExtTransportSelector;
use crate::transport::proto::nat_tcp::{NatTcpTransport, NatTcpTransportParam};
use crate::transport::proto::nat_udp::{NatUdpTransport, NatUdpTransportParam};

pub mod ctrl;
pub mod types;
mod device_handler;

pub enum NetworkStatus {
    Running,
    Stop,
}


#[derive(Clone)]
pub struct VlinkNetworkManager {
    inner: Arc<VlinkNetworkManagerInner>,
}

#[derive(Hash, PartialEq, Eq, Clone, AsRefStr, EnumString)]
pub enum ExtraProto {
    NatUdp,
    NatTcp,
}

#[derive(Clone)]
pub struct ExtraProtoStatus {
    pub endpoint: Option<String>,
    running: bool,
    error: Option<String>,
}

#[derive(Clone)]
pub struct VlinkNetworkManagerInner {
    client: Arc<VlinkClient>,
    rx: Arc<Mutex<Receiver<NetworkCtrlCmd>>>,
    secret: VlinkStaticSecret,
    device: Arc<RwLock<Option<Device>>>,
    /// 扩展协议自动选择器
    extra_selector: RwMap<String, ExtTransportSelector>,
    extra_status: RwMap<ExtraProto, ExtraProtoStatus>,
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
                extra_selector: Default::default(),
                extra_status: RwMap::new(),
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
                todo!("服务器连接失败,尝试重缓存文件读取配置");
            }
        };
        if let Some(cfg) = config {
            self.start_device(cfg).await?;
        }
        /*
         //todo 检查配置网段冲突
        */

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
                            e.set_online(true);
                        }
                    }
                    // device.insert_peer(c);
                }
                NetworkCtrlCmd::Connected => {
                    let esc = self.extra_status.clone();
                    handler::connected::handler_connected(client_c.clone(), device_c.clone(), &args, esc).await?;
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
    }

    // async fn get_device(&self) -> anyhow::Result<dyn AsRef<Device>> {
    //     self.device.read().await.ok_or(anyhow::anyhow!("device is none"))
    // }
    async fn start_device(&self, config: VlinkNetworkConfig) -> anyhow::Result<()> {
        //启动peer 协议协商
        let trans = config.transports.clone();
        let pets = config.peer_extra_transports.clone();
        let device = Device::new(config.tun_name, config.device_config).await?;

        let peers = device.peers.clone();
        let inbound_tx = device.inbound_tx();
        let mut event_bus = device.event_bus.clone();
        let mut event_rx = device.event_bus.subscribe();

        self.device.write().await.replace(device);
        //监听设备事件

        // 连接扩展端点
        //去自动选扩展端点,主动发起连接
        let mut map = HashMap::new();
        pets.into_iter().for_each(|i| {
            let e = map.entry(i.target_pub_key.clone()).or_insert(vec![]);
            e.push(i);
        });

        for (k, ps) in map {
            //switch_transport
            let key = decode_base64(k.as_str())?;
            let pub_key: [u8; 32] = key.try_into().map_err(|_| anyhow::anyhow!("error"))?;
            let peer = peers.read().unwrap().get_by_key(&pub_key);
            let inbound_tx_c = inbound_tx.clone();
            if let Some(p) = peer {
                //检测
                info!("start endpoint selector");
                {
                    let mut wr = self.extra_selector.write_lock().await;
                    let entry = wr.entry(k.to_string());

                    let selector = entry.or_insert(ExtTransportSelector::new(p, inbound_tx_c, ps));
                    //selector.insert(ps);
                }
                // self.device
                // 节点健康检测?
            };
        }


        let self_c = self.clone();


        tokio::spawn(async move {
            while let Ok(e) = event_rx.recv().await {
                if let Err(err) = device_handler::handle_device_event(self_c.clone(), e).await {
                    error!("handle device event error:{:?}", err);
                }
            }
        });

        // 启动扩展端点
        for cfg in trans {
            let txc = inbound_tx.clone();
            let cc = self.client.clone();
            let es_c = self.extra_status.clone();
            let proto = ExtraProto::from_str(cfg.proto.as_str())
                .map_err(|_| anyhow!("扩展协议:{}不支持",cfg.proto))?;
            let event_pub_c = event_bus.clone();

            //插入,管理器
            tokio::spawn(async move {
                // es_c.write_lock().await.insert(proto.clone(), ExtraProtoStatus { endpoint: None, running: true, error: None });
                debug!("start extra transport:{:?}", cfg.proto);
                if let Err(err) = start_extra_transport(cc, txc, cfg, event_pub_c).await {
                    error!("start extra transport error {err}");
                    if let Some(e) = es_c.write_lock().await.get_mut(&proto) {
                        e.running = false;
                        e.endpoint = None;
                        e.error = Some(err.to_string());
                    }
                }
            });

        }
        // 无端点和扩展端点,使用中继协议




        Ok(())
    }
}

async fn start_extra_transport(cc: Arc<VlinkClient>,
                               sender: Sender<InboundResult>,
                               cfg: TransportConfig, event_pub: DevicePublisher) -> anyhow::Result<()> {
    match cfg.proto.as_str() {
        "NatUdp" => {
            let param: NatUdpTransportParam = serde_json::from_str(&cfg.params)?;
            let mut ts = NatUdpTransport::new( sender, param, event_pub).await?;
            ts.start().await?;
        }
        "NatTcp" => {
            let param: NatTcpTransportParam = serde_json::from_str(&cfg.params)?;
            let mut ts = NatTcpTransport::new(cc, sender, param, event_pub).await?;
            ts.start().await?;
        }
        _ => {}
    }
    Ok(())
}

impl Deref for VlinkNetworkManager {
    type Target = VlinkNetworkManagerInner;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}