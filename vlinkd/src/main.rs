use clap::Parser;
use futures::Stream;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use log::{error, info};
use prost::Message;

use vlink_tun::device::config::ArgConfig;
use vlinkd::api::start_http_server;
use vlinkd::client::VlinkClient;
use vlinkd::network::{VlinkNetworkManager};
use vlinkd::network::ctrl::NetworkCtrl;
use vlinkd::storage::Storage;

mod test_route;


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
    /// 监听本地 udp 端口,服务器设置0 使用此参数
    #[arg(short, long)]
    port: Option<u16>,
    /// http 控制监听地址
    #[arg(short, long)]
    listen_addr: Option<String>,
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
    let server_addr = args.server.as_str();
    let pub_key = state.secret.base64_pub();
    // 用于客户端去控制网络
    let (ctrl, rx) = NetworkCtrl::new();
    let secret = state.secret.clone();
    let client = VlinkClient::new(server_addr.to_string(), secret.clone(), ctrl.clone());
    client.spawn().await?;
    //启动http 控制,ctrl
    start_http_server(args.listen_addr.clone(), ctrl.clone()).await?;

    let network = VlinkNetworkManager::new(client, rx, secret.clone());
    network.start(ArgConfig {
        endpoint_addr: args.endpoint_addr,
        port: args.port,
    }).await?;
    //http ctrl server

    error!("客户端关闭");
    Ok(())
}

#[cfg(test)]
pub mod test {
    use std::collections::hash_map::Entry;
    use std::time::Duration;

    // use dashmap::mapref::entry::Entry;
    use futures_util::future::join_all;
    use log::info;

    use vlink_core::rw_map::RwMap;

    #[tokio::test]
    pub async fn test() {
        // let map = Arc::new(DashMap::new());
        // map.entry()
        let map = RwMap::default();
        log4rs::init_file("/Users/huanxi/project/vlink/log4rs.yaml", Default::default()).unwrap();
        map.insert("b".to_string(), -1).await;

        let id = "a".to_string();
        let mut tasks = vec![];
        for i in 0..100 {
            let map_c = map.clone();
            let idc = id.clone();
            tasks.push(async move {
                match map_c.inter.write().await.entry(idc) {
                    Entry::Occupied(e) => {
                        // info!("o");
                    }
                    Entry::Vacant(e) => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        info!("insert {}",i);
                        e.insert(i);
                    }
                };
                // match map_c.entry(idc) {
                //     Entry::Occupied(e) => {
                //         info!("o");
                //     }
                //     Entry::Vacant(e) => {
                //         tokio::time::sleep(Duration::from_secs(1)).await;
                //         info!("insert {}",i);
                //         e.insert(i);
                //     }
                // };
            });
        }
        join_all(tasks).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}