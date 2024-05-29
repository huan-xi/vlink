use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{debug, error, info};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use crate::connect::ClientConnect;
use vlink_core::proto::pb::abi::ReqHandshake;
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use crate::network::ctrl::{NetworkCtrl, NetworkCtrlCmd};
use vlink_core::proto::pb::abi::ToClient;
use vlink_core::secret::VlinkStaticSecret;
use crate::error::ClientError;

pub struct VlinkClient {
    conn: Arc<RwLock<Option<ClientConnect>>>,
    tx: broadcast::Sender<ToClient>,
    secret: VlinkStaticSecret,
    ctrl: NetworkCtrl,
    server_addr: String,
    token: CancellationToken,
    timeout: Duration,
}

impl VlinkClient {
    pub fn new(
        server_addr: String,
        secret: VlinkStaticSecret,
        ctrl: NetworkCtrl) -> Self {
        let token = CancellationToken::new();
        Self {
            conn: Arc::new(RwLock::new(None)),
            tx: broadcast::channel::<ToClient>(10).0,
            secret,
            ctrl,
            server_addr,
            token,
            timeout: Duration::from_secs(30),
        }
    }
    /// 挂起客户端
    /// 1. 连接握手

    pub async fn spawn(&self) -> anyhow::Result<()> {
        let addr = self.server_addr.clone();
        let (etx, mut erx) = tokio::sync::mpsc::channel(1);
        //接受event loop 并监听
        let run_event_loop = async move {
            while let Some(el) = erx.recv().await {
                if let Some(e) = el {
                    if let Err(e) = e.await {
                        error!("连接断开原因{}", e);
                    }
                }
            };
            error!("event loop退出");
        };
        //重连 线程
        let ctrl_c = self.ctrl.clone();
        let txc = self.tx.clone();
        let lock_conn_c = self.conn.clone();
        let secret = self.secret.clone();
        let timeout_c = self.timeout.clone();
        let reconnect = async move {
            let mut count = 0;
            let mut if_first = true;
            while let Ok(_) = etx.send(None).await {
                let _ = etx.send(None).await;
                if count > 0 {
                    error!("3s后尝试第:{count}重新连接主服务器");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                };
                count += 1;
                let stream = match TcpStream::connect(addr.as_str()).await {
                    Ok(s) => { s }
                    Err(e) => {
                        error!("主服务器({addr})连接失败:{}", e);
                        continue;
                    }
                };
                let mut rx = txc.subscribe();
                let (conn, new_loop) = ClientConnect::new(stream, txc.clone());
                etx.send(Some(new_loop)).await.unwrap();
                info!("重新连接成功");
                count = 1;
                //接受服务器信息
                let info = timeout(timeout_c, async move {
                    let resp = rx.recv().await?;
                    match resp.to_client_data {
                        Some(ToClientData::RespServerInfo(info)) => {
                            Ok(info)
                        }
                        _ => {
                            return Err(anyhow!("握手失败,数据包错误"));
                        }
                    }
                }).await.map_err(|e| anyhow!("服务端信息错误:{}", e))??;
                debug!("server info:{:?}",info);
                let pub_key = secret.base64_pub();
                let pc = HandshakeParam {
                    pub_key,
                    sign: secret.hello_sign(BASE64_STANDARD.decode(info.key)?.as_slice().try_into()?)?,
                    token: None,
                };
                if let Err(e) = handshake(&conn, pc.clone()).await {
                    error!("握手失败:{}", e);
                    //退出
                    break;
                };
                debug!("重连握手成功");
                {
                    lock_conn_c.write().await.replace(conn);
                }
                if if_first {
                    ctrl_c.send(NetworkCtrlCmd::FirstConnected).await?;
                    if_first = false;
                }
                ctrl_c.send(NetworkCtrlCmd::Connected).await?;
            };
            Ok::<(), anyhow::Error>(())
        };


        let ctrl_c = self.ctrl.clone();
        let txc = self.tx.clone();
        // 接受服务端命令控制网络
        let process = async move {
            process_cmd(ctrl_c, txc).await
        };

        let token = self.token.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    _ = run_event_loop => {break;}
                    _ = reconnect => {break;}
                    _ = process => {break;}
                    _ = token.cancelled() => {break;}
                }
            }
        });

        Ok(())
    }

    pub async fn request(&self, data: ToServerData) -> anyhow::Result<ToClientData> {
        let conn = self.get_conn().await?;
        let data = conn.request(data).await?.ok_or(anyhow!("请求失败,返回数据为空"));
        if let Ok(ToClientData::Error(e)) = &data {
            return Err(anyhow!("请求失败:{}",e.msg));
        }
        data
    }
    async fn get_conn(&self) -> Result<ClientConnect, ClientError> {
        self.conn.read().await.clone().ok_or(ClientError::ServerNotConnected)
    }


    pub async fn send(&self, data: ToServerData) -> anyhow::Result<u64> {
        let conn = self.get_conn().await?;
        conn.send(None, data).await
    }
}

async fn process_cmd(ctrl: NetworkCtrl, txc: broadcast::Sender<ToClient>) {
    loop {
        let mut recv = txc.subscribe();
        loop {
            let data = recv.recv().await;
            if let Ok(data) = data {
                info!("处理数据:{:?}", data);
                match data.to_client_data {
                    Some(ToClientData::PeerEnter(e)) => {
                        ctrl.send(NetworkCtrlCmd::PeerEnter(e)).await.unwrap();
                    }
                    _ => {}
                }
            };
        }
    }
}

async fn process_cmd0(conn: Arc<RwLock<ClientConnect>>) {}

#[derive(Clone)]
pub struct HandshakeParam {
    pub pub_key: String,
    pub sign: String,
    pub token: Option<String>,
}

async fn connect_server() {}

/// 客户端握手
/// token(用于加入网络) or encrypt_flag(校验私钥是否正确)
async fn handshake(conn: &ClientConnect, param: HandshakeParam) -> anyhow::Result<()> {
    //私钥签名

    let resp = conn.request(ToServerData::Handshake(ReqHandshake {
        version: 0,
        pub_key: param.pub_key,
        token: param.token,
        sign: param.sign,
    })).await?;
    match resp {
        Some(ToClientData::RespHandshake(e)) => {
            if !e.success {
                return Err(anyhow!("握手失败:{}",e.msg.unwrap_or("".to_string())));
            }
        }
        _ => {
            return Err(anyhow!("握手失败,数据包错误"));
        }
    }
    Ok(())
}