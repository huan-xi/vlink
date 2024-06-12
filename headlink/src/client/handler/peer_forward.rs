use vlink_core::proto::pb::abi::{PeerForward};
use vlink_core::proto::pb::abi::peer_forward::Data;
use vlink_core::proto::pb::abi::to_client::ToClientData;
use crate::client::dispatcher::ClientRequest;
use crate::client::handler::{ExecuteResult, ToServerDataHandler};

impl ToServerDataHandler for PeerForward {
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult {
        let net = ctx.network.clone();
        if let Some(e) = self.data.as_ref() {
            match e {
                Data::RequireReply(r) => {
                    net.broadcast_to(ToClientData::RequireReply(r.clone()), vec![self.target_pub_key.clone()]).await;
                }
            }
        }
        Ok(())
    }
}