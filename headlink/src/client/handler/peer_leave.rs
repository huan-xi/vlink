use std::future::Future;
use vlink_core::proto::pb::abi::PeerLeave;
use crate::client::dispatcher::ClientRequest;
use crate::client::handler::{ExecuteResult, ToServerDataHandler};

impl ToServerDataHandler for PeerLeave {
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult {
        let net = ctx.network.clone();
        net.offline(ctx.client_id.pub_key.as_str()).await;

        Ok(())
    }
}