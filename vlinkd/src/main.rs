mod test_route;



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
use vlink_tun::DeviceConfig;
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
    #[arg(short, long)]
    tun_name: Option<String>,
    /// 服务器地址
    #[arg(short, long)]
    server: String,

    #[arg(short, long)]
    config_dir: Option<String>,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let mut args = Args::parse();
    //取目录生成秘钥对
    let state = Storage {
        path: args.config_dir.take(),
    }.load_config().await?;

    info!("stats:{:?}",state);

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
    let cfg = VlinkNetworkConfig {
        tun_name: None,
        device_config: DeviceConfig {
            private_key: state.secret.private_key.to_bytes(),
            fwmark: 0,
            port: resp_config.port as u16,
            peers: Default::default(),
            address: resp_config.address.into(),
            network: resp_config.network.into(),
            netmask: resp_config.mask as u8,
        },
    };

    let network = VlinkNetworkManager::new(client, cfg);
    network.start(rx).await?;



    error!("服务器断开");
    Ok(())
}
