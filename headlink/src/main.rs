mod client;
mod server;
mod peer;
mod api;

use clap::Parser;
use log::{error, info};
use tokio::net::TcpListener;

use futures::{AsyncReadExt, FutureExt, select, SinkExt, StreamExt};
use tokio::sync::broadcast;
use crate::client::ClientStream;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 服务器地址
    #[arg(short, long)]
    listen: Option<String>,
    #[arg(short, long)]
    db_link: Option<String>,
}

/// 流程,客户端连接
/// 握手->握手返回(服务器),超时时间 一定时间没有握手,断开连接
/// 服务器->下发配置信息
/// 根据配置信息创建tun网卡,上报状态
///
/// 服务器请求上报状态
/// 服务器请求刷新配置

/// 注册中心,用于客户端连接，下发当前ip,和公钥信息
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let args = Args::parse();
    let addr = args.listen.unwrap_or("0.0.0.0:9797".to_string());
    let tcp = TcpListener::bind(addr.as_str()).await?;
    //处理数据
    info!("Start listening on {}", addr.as_str());
    //广播器
    // let (mut tx, mut rx) = broadcast::channel(16);
    let server = server::VlinkServer::new();
    let server_c = server.clone();
    loop {
        let (stream, addr) = tcp.accept().await?;
        info!("Client: {:?} connected", addr);
        let (stream, even_loop) = ClientStream::new(stream, addr.clone(), server_c.clone());
        server_c.insert_client(stream.client.clone());
        //处理数据流
        let server_cc = server_c.clone();
        tokio::spawn(async move {
            if let Err(e) = even_loop.await {
                error!("Client Process error {:?}",e );
            }
            info!("Client: {:?} disconnected", addr);
            let cli = server_cc.remove_client(&addr).await;
            if let Some((_, mut c)) = cli {
                if let Some(c) = c.pub_key.get() {
                    server_cc.remove_peer(&c);
                }
            };
        });
    }
    error!("Server exit");

    Ok(())
}
