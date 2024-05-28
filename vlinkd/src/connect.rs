use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::anyhow;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use prost::Message;

use tokio::net::TcpStream;
use vlink_core::proto::bind_transport;
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_core::proto::pb::abi::ToServer;
use vlink_core::proto::pb::abi::ToClient;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use tokio::sync::{broadcast, mpsc, oneshot};
use bytes::{Bytes, BytesMut};
use log::{debug, error, info};
use tokio::select;
use tokio::time::timeout;

type ToServerParam = (Option<u64>, ToServerData, oneshot::Sender<Result<u64, std::io::Error>>);

#[derive(Clone)]
pub struct ClientConnect {
    // id: AtomicU64,
    stream: broadcast::Sender<ToClient>,
    to_server_tx: mpsc::Sender<ToServerParam>,
    timeout: Duration,
    is_connected: Arc<AtomicBool>,
}


pub struct ClientRequest {}

impl ClientConnect {
    pub fn new(stream: TcpStream, tx: broadcast::Sender<ToClient>) -> (ClientConnect, impl Future<Output=anyhow::Result<()>> + Sized) {
        let (mut sink, mut stream) = bind_transport(stream).split();

        let (to_server_tx, mut to_server_rx) = mpsc::channel::<ToServerParam>(10);
        let is_connected = Arc::new(AtomicBool::new(true));
        let is_connected_c = is_connected.clone();
        let to_server_handler = async move {
            let mut id = 0;
            while let Some((id_op, data, tx)) = to_server_rx.recv().await {
                let use_id = id_op.unwrap_or_else(|| {
                    id = id + 1;
                    id
                });
                let req = ToServer {
                    id: use_id,
                    to_server_data: Some(data),
                };
                let mut bytes = BytesMut::new();
                req.encode(&mut bytes)?;
                let result = sink.send(Bytes::from(bytes))
                    .await
                    .map(|_| id);
                let _ = tx.send(result);
            }
            Ok::<(), anyhow::Error>(())
        };
        let tx_c = tx.clone();
        let recv_handler = async move {
            while let Some(Ok(bytes)) = stream.next().await {
                let data = ToClient::decode(bytes.as_ref())?;
                let _ = tx_c.send(data);
            }
            Ok::<(), anyhow::Error>(())
        };


        //event_loop
        let event_loop = async move {
            is_connected_c.store(true, Ordering::SeqCst);
            loop {
                select! {
                    _ = recv_handler => {break;}
                    e = to_server_handler => {
                        error!("to_server_handler {:?}", e);
                        break;
                    }
                }
            }
            is_connected_c.store(false, Ordering::SeqCst);
            Ok::<(), anyhow::Error>(())
        };


        //tx->sink
        //stream->tx2
        (ClientConnect {
            stream: tx,
            to_server_tx,
            timeout: Duration::from_secs(1),
            is_connected,
        }, event_loop)
    }

    /// 发送一个请求并等待结果
    pub async fn request(&self, data: ToServerData) -> anyhow::Result<Option<ToClientData>> {
        if !self.is_connected.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("客户端已断开连接"));
        }
        let mut rx = self.stream.subscribe();
        let id = self.send(None, data).await?;
        //rx 等待结果
        timeout(self.timeout, async {
            while let data = rx.recv().await? {
                if data.id == id {
                    return Ok(data.to_client_data);
                }
            };
            Err(anyhow!("读取错误"))
        }).await.map_err(|_| anyhow!("调用超时"))?
    }


    pub async fn send(&self, id: Option<u64>, data: ToServerData) -> anyhow::Result<u64> {
        if !self.is_connected.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("客户端已断开连接"));
        }
        timeout(self.timeout, async {
            let (otx, orx) = oneshot::channel();
            self.to_server_tx.send((id, data, otx)).await?;
            let id = orx.await??;
            Ok(id)
        }).await.map_err(|_| anyhow!("调用超时"))?
    }
}


#[cfg(test)]
mod test {
    use tokio::sync::broadcast;

    #[test]
    pub fn test2() {
        println!("hello");
    }
}