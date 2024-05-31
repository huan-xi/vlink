use sea_orm::ColIdx;
use vlink_core::proto::pb::abi::DevHandshakeComplete;
use crate::client::dispatcher::ClientRequest;
use crate::client::handler::{ExecuteResult, ToServerDataHandler};
use crate::client::handler::helpers::union_pub_key;
use crate::network::PeerConnect;


impl ToServerDataHandler for DevHandshakeComplete {
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult {
        let network = ctx.network.clone();
        let (key, direction) = union_pub_key(ctx.pub_key().as_str(), self.target_pub_key.as_str());
        network.connects.write_lock().await.insert(key, PeerConnect {
            direction,
            proto: self.proto.clone(),
        });
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    #[test]
    pub fn test_union_pub_key() {
        let (a1, b) = super::union_pub_key("a", "b");
        let (a2, b) = super::union_pub_key("b", "a");
        println!("a1: {}, a2: {}", a1, a2);
        assert_eq!(a1, a2);
    }
}