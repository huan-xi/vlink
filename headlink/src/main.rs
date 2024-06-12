use clap::Parser;
use log::{error, info};
use tokio::net::TcpListener;
use headlink::db::init::open_db;
use headlink::server;
use headlink::client::ClientStream;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 服务器地址
    #[arg(short, long)]
    listen: Option<String>,
    /// 数据库连接
    #[arg(short, long)]
    db_schema: Option<String>,
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
    let conn = open_db(args.db_schema.expect("db schema不能为空").as_str()).await;


    //广播器
    // let (mut tx, mut rx) = broadcast::channel(16);
    let server = server::VlinkServer::new(conn).await?;
    let server_c = server.clone();
    loop {
        info!("start accept");
        let (stream, addr) = tcp.accept().await?;
        info!("Client: {:?} connected", addr);
        let (stream, even_loop) = ClientStream::new(stream, addr.clone(), server_c.clone());
        let server_cc = server_c.clone();
        tokio::spawn(async move {
            server_cc.insert_client(stream.client.clone()).await;
            if let Err(e) = even_loop.await {
                error!("Client Process error {:?}",e );
            }
            info!("Client: {:?} disconnected", addr);
            let cli = server_cc.remove_client(&addr).await;
            if let Some(c) = cli {
                if let Some(c) = c.client_id.get() {
                    if let Ok(network) = server_cc.get_network(c.network_id).await {
                        network.offline(&c.pub_key).await;
                    }
                }
            };
        });
    }
    error!("Server exit");

    Ok(())
}