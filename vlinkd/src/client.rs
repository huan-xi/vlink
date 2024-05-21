use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use futures_util::{StreamExt, TryFutureExt};
use log::{error, info};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use crate::connect::ClientConnect;
use vlink_core::proto::pb::abi::ReqHandshake;
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use crate::network::{NetworkCtrl, NetworkCtrlCmd};
use vlink_core::proto::pb::abi::ToClient;

pub struct VlinkClient {
    conn: Arc<RwLock<ClientConnect>>,
    tx: broadcast::Sender<ToClient>,
}

impl VlinkClient {
    pub async fn spawn(addr: &str, param: HandshakeParam, ctrl: NetworkCtrl) -> anyhow::Result<Self> {
        //开启连接
        let stream = TcpStream::connect(addr).await?;
        let (tx, _) = broadcast::channel::<ToClient>(10);

        let (conn, even_loop) = ClientConnect::new(stream, tx.clone());
        let token = CancellationToken::new();
        let lock_conn = Arc::new(RwLock::new(conn));
        let lock_conn_c = lock_conn.clone();
        let addr = addr.to_string();
        let (etx, mut erx) = tokio::sync::mpsc::channel(1);
        etx.send(Some(even_loop)).await?;


        let run_event_loop = async move {
            while let Some(el) = erx.recv().await {
                if let Some(e) = el {
                    if let Err(e) = e.await {
                        error!("连接断开原因{}", e);
                    }
                }
            };
        };

        //重连
        let ctrl_c = ctrl.clone();
        let txc = tx.clone();
        let pc = param.clone();
        let reconnect = async move {
            while let Ok(_) = etx.send(None).await {
                let _ = etx.send(None).await;
                error!("3s后尝试重新连接");
                tokio::time::sleep(Duration::from_secs(3)).await;
                let stream = match TcpStream::connect(addr.as_str()).await {
                    Ok(s) => { s }
                    Err(_) => {
                        continue;
                    }
                };
                let (conn, new_loop) = ClientConnect::new(stream, txc.clone());
                // even_loop = Some(new_loop);
                info!("重新连接成功");
                etx.send(Some(new_loop)).await.unwrap();
                if let Err(e) = handshake(&conn, pc.clone()).await {
                    error!("握手失败:{}", e);
                    //退出
                    break;
                };
                info!("重连握手成功");
                {
                    *lock_conn_c.write().await = conn;
                }
                ctrl_c.send(NetworkCtrlCmd::Reenter).await?;
            };
            Ok::<(), anyhow::Error>(())
        };
        let lock_conn_c = lock_conn.clone();


        let ctrl_c = ctrl.clone();
        let txc = tx.clone();
        let process = async move {
            process_cmd(lock_conn_c, ctrl_c, txc).await
        };

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

        handshake(lock_conn.read().await.deref(), param).await?;


        Ok(
            Self { conn: lock_conn, tx }
        )
    }

    pub async fn request(&self, data: ToServerData) -> anyhow::Result<ToClientData> {
        let conn = self.conn.read().await;
        let data = conn.request(data).await?.ok_or(anyhow!("请求失败,返回数据为空"));
        if let Ok(ToClientData::Error(e) )= &data {
            return Err(anyhow!("请求失败:{}",e.msg));
        }
        data
    }


    pub async fn send(&self, data: ToServerData) -> anyhow::Result<u64> {
        let conn = self.conn.read().await;
        conn.send(None, data).await
    }
}

async fn process_cmd(conn: Arc<RwLock<ClientConnect>>, ctrl: NetworkCtrl, txc: broadcast::Sender<ToClient>) {
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
    pub token: String,
}

/// 客户端握手
/// token(用于加入网络) or encrypt_flag(校验私钥是否正确)
async fn handshake(conn: &ClientConnect, param: HandshakeParam) -> anyhow::Result<()> {
    //私钥签名

    let resp = conn.request(ToServerData::Handshake(ReqHandshake {
        version: 0,
        pub_key: param.pub_key,
        token: param.token,
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