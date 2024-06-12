mod dispatcher;
pub mod handler;
pub mod error;

use sea_orm::ColumnTrait;
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::{anyhow, Error};
use bytes::{Bytes, BytesMut};
use vlink_core::proto::pb::abi::*;
use futures::{SinkExt, Stream, StreamExt};
use log::{debug, error, info};
use vlink_core::proto::pb::abi::RespConfig;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::select;
use vlink_core::proto::pb::abi::to_server::ToServerData;
use vlink_core::proto::pb::abi::to_client::*;
use vlink_core::proto::pb::abi::ToClient;
use vlink_core::proto::pb::abi::RespHandshake;
use vlink_core::proto::pb::abi::ToServer;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use crate::server::VlinkServer;
use vlink_core::proto::bind_transport;
use futures::FutureExt;
use futures_util::future::join_all;
use prost::Message;
use sea_orm::{EntityTrait, QueryFilter};
use tokio::time::timeout;
use crate::db::entity::prelude::{NetworkEntity, NetworkTokenColumn, NetworkTokenEntity, PeerColumn, PeerEntity};
use crate::client::dispatcher::{Dispatcher, ClientRequest, RequestContext};
use crate::peer::VlinkPeer;

pub type ToClientParam = (Option<u64>, ToClientData, oneshot::Sender<Result<u64, std::io::Error>>);

#[derive(Clone, Debug)]
pub struct ClientId {
    pub pub_key: String,
    pub network_id: i64,
}

/// 客户端连接
#[derive(Clone)]
pub struct ClientConnect {
    pub addr: SocketAddr,
    pub client_id: Arc<OnceLock<ClientId>>,
    sender: mpsc::Sender<ToClientParam>,
}

impl ClientConnect {
    pub async fn send(&self, id: Option<u64>, data: ToClientData) -> anyhow::Result<u64> {
        let (tx, rx) = oneshot::channel();
        self.sender.send((id, data, tx)).await?;
        let id = rx.await??;
        Ok(id)
    }
    pub async fn close(&self) {
        self.sender.closed().await
    }
    pub fn client_id(&self) -> Option<ClientId> {
        self.client_id.get().cloned()
    }
}

pub struct Context {}

/// 一个客户端连接

pub struct ClientStream {
    // inner: Option<S>,
    pub server: VlinkServer,
    // receiver: Option<mpsc::Receiver<Vec<u8>>>,
    pub client: ClientConnect,
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
        let client = ClientConnect {
            addr,
            client_id: Arc::new(Default::default()),
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
        let process = process_client(server.clone(), client.clone(), recv_tx.clone(), recv_rx);

        let event_loop = async move {
            is_connected_c.store(true, Ordering::SeqCst);
            loop {
                let resp = select! {
                    resp = recv_handler => {resp}
                    resp = to_client_handler => {resp}
                    resp = process => {resp}
                };
                error!("process error:{:?}",resp);
                is_connected_c.store(false, Ordering::SeqCst);
                return resp;
            }
        };

        (Self {
            server,
            client,
            is_connected,
            recv_tx,
        }, event_loop)
    }
}

/// 循环处理客户端数据
async fn process_client(server: VlinkServer, client: ClientConnect, tx: broadcast::Sender<ToServer>, recv_rx: broadcast::Receiver<ToServer>) -> anyhow::Result<()> {
    let client_id = await_handshake(server.clone(), 10, client.clone(), recv_rx).await?;
    let mut recv = tx.subscribe();
    debug!("握手成功,clientId:{:?}",client_id);
    let network = server.get_network(client_id.network_id).await?;
    let ctx = Arc::new(RequestContext {
        client_id: client_id.clone(),
        server: server.clone(),
        client: client.clone(),
        network,
    });
    let dispatcher = Dispatcher::new();
    debug!("握手成功,开始接受数据:{:?}",client_id);
    while let Ok(data) = recv.recv().await {
        info!("处理数据:{:?}",data);
        let id = data.id;
        if let Some(data) = data.to_server_data {
            // data.hande();
            if let Err(e) = dispatcher.dispatch(ClientRequest {
                id,
                ctx: ctx.clone(),
            }, data).await {
                // 发送错误
                error!("处理数据错误:{}",e);
                client.send(Some(id), ToClientData::Error(ToClientError {
                    code: e.code(),
                    msg: e.to_string(),
                })).await?;
            }
        } else {
            error!("数据错误:pub:{}",client_id.pub_key);
        }
    };
    Ok(())
}


//<T: AsyncRead + AsyncWrite>(stream: T) where <T as Stream>::Item: Vec<u8>
/// 握手成功返回pub_key
async fn await_handshake(server: VlinkServer, secs: u64, client: ClientConnect, mut rx: broadcast::Receiver<ToServer>) -> anyhow::Result<ClientId> {
    let info = server.info.clone();
    client.send(None, ToClientData::RespServerInfo(RespServerInfo {
        version: info.version,
        key: info.secret.base64_pub(),
        desc: None,
    })).await?;
    timeout(Duration::from_secs(secs), async {
        if let Ok(data) = rx.recv().await {
            let id = data.id;
            let result = handshake0(&server, &client, data).await;
            client.send(Some(id), ToClientData::RespHandshake(RespHandshake { success: result.is_ok(), msg: result.as_ref().err().map(|e| e.to_string()) })).await?;
            return result;
        }
        Err(anyhow!("握手包处理错误"))
    }).await.map_err(|_| anyhow!("握手超时"))?
}


async fn handshake0(server: &VlinkServer, client: &ClientConnect, data: ToServer) -> anyhow::Result<ClientId> {
    if let Some(ToServerData::Handshake(data)) = data.to_server_data {
        debug!("握手包数据:{:?}",data);
        let pub_key = data.pub_key.clone();

        let network_id = if let Some(token) = data.token.clone() {
            let token = NetworkTokenEntity::find()
                .filter(NetworkTokenColumn::Token.eq(data.token.unwrap().as_str()))
                .one(server.conn())
                .await?
                .ok_or(anyhow!("token不存在"))?;
            if token.disabled {
                return Err(anyhow!("token已禁用"));
            }
            let network = server.get_network(token.network_id).await?;
            network.network_id
        } else {
            //peer取
            let peer = PeerEntity::find()
                .filter(PeerColumn::PubKey.eq(pub_key.as_str()))
                .one(server.conn())
                .await?
                .ok_or(anyhow!("peer 未注册"))?;
            peer.network_id
        };
        let pub_key_c = pub_key.clone();
        // 从server中 取网络
        let client_id = ClientId {
            pub_key,
            network_id,
        };
        client.client_id.set(client_id.clone()).map_err(|e| anyhow!("set error"))?;
        //发送握手成功
        let network = server.get_network(network_id).await?;

        if let Some(e) = network.peers.read_lock().await.get(pub_key_c.as_str()) {
            if e.is_online() {
                let err = format!("peer已连接,pub({})", pub_key_c.as_str());
                return Err(anyhow!(err));
            }
        }
        // client.send(Some(id), ToClientData::RespHandshake(RespHandshake { success: true, msg: None })).await?;
        return Ok(client_id);
    } else {
        Err(anyhow!("握手包数据错误"))
    }
}