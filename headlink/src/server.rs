use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use dashmap::DashMap;
use crate::client::ClientConnect;
use crate::peer::VlinkPeer;

pub struct ServerInner {
    clients: DashMap<SocketAddr, ClientConnect>,
    pub peers: DashMap<String, VlinkPeer>,
}

impl ServerInner {
    pub fn insert_client(&self, client: ClientConnect) {
        self.clients.insert(client.addr.clone(), client);
    }
    pub async fn remove_client(&self, addr: &SocketAddr) -> Option<(SocketAddr, ClientConnect)> {
        let client = self.clients.remove(addr);
        if let Some((_, client)) = &client {
            client.close().await
        }
        client
    }
    pub fn get_client(&self, addr: &SocketAddr) -> Option<ClientConnect> {
        self.clients.get(addr).map(|v| v.value().clone())
    }

    pub fn remove_peer(&self, pub_key: &str) {
        self.peers.remove(pub_key);
    }


}


#[derive(Clone)]
pub struct VlinkServer {
    inner: Arc<ServerInner>,
}

impl VlinkServer {
    pub fn new() -> VlinkServer {
        VlinkServer {
            inner: Arc::new(ServerInner { clients: Default::default(), peers: Default::default() })
        }
    }
}

impl Deref for VlinkServer {
    type Target = ServerInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}