use std::ops::Deref;
use std::sync::Arc;
use log::error;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use crate::network::VlinkNetwork;
use crate::client::handler::{ExecuteResult, ToServerDataHandler};
use crate::client::{ClientConnect, ClientId, ToServerData};
use crate::client::error::ExecuteError;
use crate::server::VlinkServer;



pub struct Dispatcher {}

pub struct ClientRequest {
    /// 请求事务id
    pub(crate) id: u64,
    pub ctx: Arc<RequestContext>,
}

impl ClientRequest {
    pub async fn send_resp(&self, resp: ToClientData) -> anyhow::Result<u64> {
        self.client.send(Some(self.id), resp).await
    }
}

impl Deref for ClientRequest {
    type Target = RequestContext;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

pub struct RequestContext {
    pub(crate) client_id: ClientId,
    pub(crate) server: VlinkServer,
    pub(crate) client: ClientConnect,
    pub(crate) network: VlinkNetwork,
}

impl RequestContext {
    pub fn conn(&self) -> &sea_orm::DatabaseConnection {
        self.server.conn()
    }
    pub fn pub_key(&self) -> String {
        self.client_id.pub_key.clone()
    }
}

impl Dispatcher {
    pub fn new() -> Dispatcher {
        Dispatcher {}
    }

    pub async fn dispatch(&self, ctx: ClientRequest, data: ToServerData) -> ExecuteResult {
        match data {
            ToServerData::PeerEnter(data) => data.execute(ctx).await,
            ToServerData::ReqConfig(data) => data.execute(ctx).await,
            _ => {
                error!("Dispatcher::dispatch: unknown data type");
                return Err(ExecuteError::StrMessage("Dispatcher::dispatch: unknown data type"));
            }
        }
    }
}
