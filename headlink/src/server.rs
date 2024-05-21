use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use anyhow::anyhow;
use log::debug;
use sea_orm::{DatabaseConnection, EntityTrait};
use crate::db::entity::prelude::{NetworkEntity, PeerColumn, PeerEntity, PeerModel};
use crate::network::{VlinkNetwork, VlinkNetworkInner};
use crate::client::ClientConnect;
use crate::peer::VlinkPeer;
use sea_orm::*;
use vlink_core::rw_map::RwMap;

pub struct ServerInner {
    clients: RwMap<SocketAddr, ClientConnect>,
    // pub peers: Peers,
    conn: DatabaseConnection,
    pub networks: RwMap<i64, VlinkNetwork>,
    // pub networks: DashMap<i64, VlinkNetwork>,
}



#[derive(Clone, Default)]
pub struct Peers {
    // peers: Arc<DashMap<String, VlinkPeer>>,
    peers: RwMap<String, VlinkPeer>,
}

impl Peers {
    pub fn new(peers: Vec<VlinkPeer>) -> Peers {
        let peers:Vec<(String,VlinkPeer)> = peers.into_iter()
            .map(|p| (p.model.pub_key.clone(), p))
            .collect();
        Peers {
            peers: RwMap::from(peers)
        }
    }
    pub async fn refresh_model(&self, model: PeerModel) {
        let pub_key: &str = model.pub_key.as_str();
        if let Some(mut p) = self.peers.write_lock().await.get_mut(pub_key) {
            p.model = model;
        }
    }
}

impl Deref for Peers {
    type Target = RwMap<String, VlinkPeer>;
    fn deref(&self) -> &Self::Target {
        &self.peers
    }
}


impl ServerInner {
    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }
    pub fn insert_client(&self, client: ClientConnect) {
        self.clients.insert(client.addr.clone(), client);
    }
    pub async fn remove_client(&self, addr: &SocketAddr) -> Option<ClientConnect> {
        let client = self.clients.remove(addr).await;
        if let Some(client) = &client {
            client.close().await
        }
        client
    }
    pub async fn get_client(&self, addr: &SocketAddr) -> Option<ClientConnect> {
        self.clients.read_lock().await.get(addr).map(|v| v.clone())
    }


    pub async fn get_network(&self, network_id: i64) -> anyhow::Result<VlinkNetwork> {

        Ok(match self.networks.write_lock().await.entry(network_id) {
            Entry::Occupied(e) => { e.get().clone() }
            Entry::Vacant(e) => {
                debug!("get network from db");
                let network = NetworkEntity::find_by_id(network_id)
                    .one(self.conn())
                    .await?
                    .ok_or(anyhow!("网络id不存在"))?;
                //查peers
                let mut peers = PeerEntity::find()
                    .filter(PeerColumn::NetworkId.eq(network.network_id))
                    .all(self.conn())
                    .await?;
                let peers: Vec<VlinkPeer> = peers.into_iter()
                    .map(|p| VlinkPeer::from(p))
                    .collect();

                let network = VlinkNetwork {
                    inner: Arc::new(VlinkNetworkInner {
                        network_id: network.network_id,
                        cidr: network.cidr.parse()?,
                        peers: Peers::new(peers),
                    }),
                };
                // 查询peers
                e.insert(network.clone());
                network
            }
        })
    }
}


#[derive(Clone)]
pub struct VlinkServer {
    inner: Arc<ServerInner>,
}

impl VlinkServer {
    pub fn new(conn: DatabaseConnection) -> VlinkServer {
        VlinkServer {
            inner: Arc::new(ServerInner { clients: Default::default(), conn,
                networks: Default::default() })
        }
    }
}

impl Deref for VlinkServer {
    type Target = ServerInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}