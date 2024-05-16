use std::cell::OnceCell;
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;
use anyhow::{anyhow, Error};
use bytes::{Bytes, BytesMut};
use futures::{SinkExt, Stream, StreamExt};
use log::{debug, error, info};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::select;
use core::proto::pb::abi::to_server::ToServerData;
use core::proto::pb::abi::to_client::ToClientData;
use core::proto::pb::abi::ToClient;
use core::proto::pb::abi::RespHandshake;
use core::proto::pb::abi::ToServer;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use crate::server::VlinkServer;
use core::proto::bind_transport;
use futures::FutureExt;
use prost::Message;
use tokio::time::timeout;

pub type ToClientParam = (Option<u64>, ToClientData, oneshot::Sender<Result<u64, std::io::Error>>);

#[derive(Clone)]
pub struct VlinkClient {
    pub addr: SocketAddr,
    pub pub_key: Arc<OnceLock<String>>,
    pub sender: mpsc::Sender<ToClientParam>,
}

impl VlinkClient {
    pub async fn send(&self, id: Option<u64>, data: ToClientData) -> anyhow::Result<u64> {
        let (tx, rx) = oneshot::channel();
        self.sender.send((id, data, tx)).await?;
        let id = rx.await??;
        Ok(id)
    }
}

pub struct Context {}

/// 一个客户端连接

pub struct ClientStream {
    // inner: Option<S>,
    pub server: VlinkServer,
    // receiver: Option<mpsc::Receiver<Vec<u8>>>,
    pub(crate) client: VlinkClient,
    is_connected: Arc<AtomicBool>,
    recv_tx: broadcast::Sender<ToServer>,
}

impl ClientStream {
    pub fn new<S: AsyncRead + AsyncWrite + Unpin + Send>(stream: S,
                                                         addr: SocketAddr,
                                                         server: VlinkServer,
    ) -> (ClientStream, impl Future<Output=Result<(), Error>> + Sized) {
        let (tx, mut rx) = mpsc::channel(128);
        let (recv_tx, recv_rx) = broadcast::channel::<ToServer>(128);
        let is_connected = Arc::new(AtomicBool::new(true));
        let is_connected_c = is_connected.clone();
        //addr,
        let client = VlinkClient {
            addr,
            pub_key: Arc::new(Default::default()),
            sender: tx,
        };
        let (mut sink, mut stream) = bind_transport(stream).split();

        //开启数据交换
        let to_client_handler = async move {
            let mut id = 0;
            while let Some((id_op, data, tx)) = rx.recv().await {
                let use_id = id_op.unwrap_or_else(|| {
                    id = id + 1;
                    id
                });
                let req = ToClient {
                    id: use_id,
                    to_client_data: Some(data),
                };
                let mut bytes = BytesMut::new();
                req.encode(&mut bytes)?;
                let result = sink.send(Bytes::from(bytes)).await
                    .map(|_| id);
                let _ = tx.send(result);
            };
            Ok::<(), anyhow::Error>(())
        };

        let tx_c = recv_tx.clone();
        let recv_handler = async move {
            while let Some(Ok(bytes)) = stream.next().await {
                let data = ToServer::decode(bytes.as_ref())?;
                let _ = tx_c.send(data);
            };
            Ok::<(), anyhow::Error>(())
        };
        //process_handler
        let process = process_handler(client.clone(), recv_tx.clone(), recv_rx);

        let event_loop = async move {
            is_connected_c.store(true, Ordering::SeqCst);
            loop {
                select! {
                    _ = recv_handler => {break;}
                    _ = to_client_handler => {break;}
                    resp = process => {
                        return resp;
                    }
                }
            }
            is_connected_c.store(false, Ordering::SeqCst);
            Ok::<(), anyhow::Error>(())
        };

        (Self {
            server,
            client,
            is_connected,
            recv_tx,
        }, event_loop)
    }
    async fn handel_frame(&self, bytes: bytes::BytesMut) -> anyhow::Result<()> {
        let req = ToServer::decode(bytes.as_ref())?;
        info!("recv data:{:?}", req);
        if let Some(req) = req.to_server_data {
            match req {
                ToServerData::ReqConfig(data) => {}
                ToServerData::Handshake(data) => {
                    // self.client.is_handshake.store(true, std::sync::atomic::Ordering::SeqCst);
                    info!("握手成功,{data:?}");
                    let ipv4: u32 = Ipv4Addr::new(10, 2, 0, 2).into();
                    info!("ipv4:{:?}", ipv4);

                    // 发送客户端配置信息
                    /*self.client.send(ToClientData::JoinNetwork(ReqNetwork {
                        network_id: 1,
                        address: ipv4,
                        mask: 0,
                        ipv6_addr: None,
                    })).await?;*/
                }
            }
        }
        Ok(())
    }

    /// 循环处理客户端数据
    pub async fn process(mut self) -> anyhow::Result<()> {
        //启动Event loop


        Ok(())
    }
}

async fn process_handler(client: VlinkClient, tx: broadcast::Sender<ToServer>, recv_rx: broadcast::Receiver<ToServer>) -> anyhow::Result<()> {
    let mut recv = tx.subscribe();
    let pubkey = await_handshake(10, client.clone(), recv_rx).await?;
    debug!("握手成功,开始接受数据:{:?}",pubkey);
    while let Ok(data) = recv.recv().await {
        info!("处理数据:{:?}",data);
        let id = data.id;
        if let Some(data) = data.to_server_data {
            match data {
                ToServerData::ReqConfig(_) => {
                    //下发配置
                }
                _ => {}
            }
        }
    };
    Ok(())
}

//<T: AsyncRead + AsyncWrite>(stream: T) where <T as Stream>::Item: Vec<u8>
async fn await_handshake(secs: u64, client: VlinkClient, mut rx: broadcast::Receiver<ToServer>) -> anyhow::Result<String> {
    timeout(Duration::from_secs(secs), async {
        while let Ok(data) = rx.recv().await {
            let id = data.id;
            if let Some(ToServerData::Handshake(data)) = data.to_server_data {
                debug!("握手包数据:{:?}",data);
                let pub_key = data.pub_key;
                client.pub_key.set(pub_key.clone())
                    .map_err(|e| anyhow!("set error"))?;
                //发送握手成功
                client.send(Some(id), ToClientData::Handshake(RespHandshake { success: true, msg: None })).await?;
                return Ok(pub_key);
            };
        }
        Err(anyhow!("握手包处理错误"))
    }).await.map_err(|_| anyhow!("握手超时"))?
}