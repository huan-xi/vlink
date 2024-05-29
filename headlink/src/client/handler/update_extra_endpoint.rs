use vlink_core::proto::pb::abi::UpdateExtraEndpoint;
use crate::client::dispatcher::ClientRequest;
use crate::client::handler::{ExecuteResult, ToServerDataHandler};

impl ToServerDataHandler for UpdateExtraEndpoint{
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult {
        let network = ctx.network.clone();



        Ok(())
    }
}