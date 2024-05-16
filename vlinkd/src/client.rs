use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use futures_util::StreamExt;
use log::{error, info};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use crate::connect::ClientConnect;
use core::proto::pb::abi::ReqHandshake;
use core::proto::pb::abi::to_server::ToServerData;
use core::proto::pb::abi::to_client::ToClientData;
use crate::network::{NetworkCtrl, NetworkCtrlCmd};

pub struct VlinkClient {
    conn: Arc<RwLock<ClientConnect>>,
}

impl VlinkClient {
    pub async fn spawn(addr: &str, pub_key: &str, ctrl: NetworkCtrl) -> anyhow::Result<Self> {
        //开启连接
        let stream = TcpStream::connect(addr).await?;
        let (conn, even_loop) = ClientConnect::new(stream);
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
        let pub_key_c = pub_key.to_string();

        //重连
        let ctrl_c = ctrl.clone();
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
                let (conn, new_loop) = ClientConnect::new(stream);
                // even_loop = Some(new_loop);
                info!("重新连接成功");
                etx.send(Some(new_loop)).await.unwrap();
                if let Err(e) = handshake(&conn, pub_key_c.as_str()).await {
                    error!("握手失败:{}", e);
                    //退出
                    break;
                };
                info!("重连握手成功");
                *lock_conn_c.write().await = conn;
                ctrl_c.send(NetworkCtrlCmd::Reenter).await?;
            };
            Ok::<(), anyhow::Error>(())
        };
        let lock_conn_c = lock_conn.clone();
        let ctrl_c = ctrl.clone();
        let process = async move {
            process_cmd(lock_conn_c,ctrl_c).await
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

        handshake(lock_conn.read().await.deref(), pub_key).await?;


        Ok(
            Self { conn: lock_conn }
        )
    }

    pub async fn request(&self, data: ToServerData) -> anyhow::Result<Option<ToClientData>> {
        let conn = self.conn.read().await;
        conn.request(data).await
    }
    pub async fn send(&self, data: ToServerData) -> anyhow::Result<u64> {
        let conn = self.conn.read().await;
        conn.send(None, data).await
    }
}

async fn process_cmd(conn: Arc<RwLock<ClientConnect>>, ctrl: NetworkCtrl) {
    loop {
        let mut recv = conn.read().await.receiver();
        loop {
            let data = recv.recv().await;
            if let Ok(data) = data {
                info!("处理数据:{:?}", data);
                match data.to_client_data {
                    Some(ToClientData::PeerEnter(e))=>{
                        ctrl.send(NetworkCtrlCmd::PeerEnter(e)).await.unwrap();
                    }
                    _ => {

                    }
                }
            };
        }
    }
}

async fn process_cmd0(conn: Arc<RwLock<ClientConnect>>) {}

async fn handshake(conn: &ClientConnect, pub_key: &str) -> anyhow::Result<()> {
    let resp = conn.request(ToServerData::Handshake(ReqHandshake {
        version: 0,
        pub_key: pub_key.to_string(),
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