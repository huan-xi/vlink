mod test_route;


use std::collections::HashSet;
use std::net::{SocketAddr, SocketAddrV4};
use std::time::Duration;
use anyhow::anyhow;
use futures::Stream;

use futures_util::{SinkExt, StreamExt, TryStreamExt};
use log::{debug, error, info};
use prost::Message;
use clap::Parser;
use rand_core::OsRng;
use tokio::time;
use x25519_dalek::StaticSecret;

use core::proto::pb::abi::*;

use core::proto::pb::abi::to_client::*;
use vlink_tun::{DeviceConfig, PeerConfig};
use vlink_tun::device::config::ArgConfig;
use vlink_tun::device::peer::cidr::Cidr;
use vlinkd::client::VlinkClient;
use vlinkd::network::config::VlinkNetworkConfig;
use vlinkd::network::{NetworkCtrl, VlinkNetworkManager};
use vlinkd::storage::Storage;
use crate::to_server::ToServerData;

/// 一个客户端守护进程
/// 一个vlinkd 管理一个tun接口,如果需要接入两个vlink 网络请启动两个进程
/// 先取本地缓存,如果有网络信息,则开启网络，否则想服务端请求网络配置信息,连不上服务器也能通过本地缓存建立连接?
/// 保证服务器必须能连上
///
/// 发送握手请求,与服务端建立连接
///

///
/// 等待接受服务端指令
///
///
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// tun 网卡名称
    #[arg(long)]
    tun_name: Option<String>,
    /// 服务器地址
    #[arg(short, long)]
    server: String,
    /// 数据目录配置
    #[arg(short, long)]
    config_dir: Option<String>,
    /// 加入主机名
    #[arg(long)]
    hostname: Option<String>,
    /// 服务器连接 token
    #[arg(short, long)]
    token: Option<String>,
    /// 连接端点地址
    #[arg(short, long)]
    endpoint_addr: Option<String>,
    /// 监听本地 udp 端口,为空则随机
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let mut args = Args::parse();
    //取目录生成秘钥对
    let state = Storage {
        path: args.config_dir.take(),
    }.load_config().await?;

    info!("state:{:?}",state);

    let addr = args.server.as_str();

    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let ctrl = NetworkCtrl { sender: tx };

    let client = match VlinkClient::spawn(addr, state.secret.base64_pub().as_str(), ctrl).await {
        Ok(c) => { c }
        Err(e) => {
            error!("连接服务器失败:{}",e);
            return Ok(());
        }
    };


    let resp = client.request(ToServerData::ReqConfig(ReqConfig {})).await?;
    info!("resp:{:?}",resp);
    let resp_config = if let Some(ToClientData::RespConfig(config)) = resp {
        config
    } else {
        return Err(anyhow!("响应错误"));
    };

    let mut device_config = DeviceConfig {
        private_key: state.secret.private_key.to_bytes(),
        fwmark: 0,
        port: {
            if resp_config.port > 0 {
                resp_config.port as u16
            } else {
                args.port.unwrap_or(0)
            }
        },
        peers: Default::default(),
        address: resp_config.address.into(),
        network: resp_config.network.into(),
        netmask: resp_config.mask as u8,
    };
    for p in resp_config.peers.iter() {
        let pk = core::base64::decode_base64(p.pub_key.as_str())?;
        let mut allowed_ips = HashSet::new();
        allowed_ips.insert(Cidr::new(p.ip.parse().unwrap(), 32));
        device_config = device_config.peer(PeerConfig {
            public_key: pk.try_into().unwrap(),
            allowed_ips,
            endpoint: match p.endpoint_addr.clone() {
                None => {None}
                Some(addr) => {
                    //Some(SocketAddr::V4(SocketAddrV4::new(e.endpoint_addr.parse().unwrap(), e.port as u16)))
                    Some(SocketAddr::new(addr.parse()?, p.port as u16))
                }
            },
            preshared_key: None,
            lazy: false,
            no_encrypt: false,
            persistent_keepalive: None,
        });
    }

    let cfg = VlinkNetworkConfig {
        tun_name: None,
        device_config,
        arg_config: ArgConfig {
            endpoint_addr: args.endpoint_addr,
        },
    };

    let network = VlinkNetworkManager::new(client, cfg);
    network.start(rx).await?;


    error!("服务器断开");
    Ok(())
}
