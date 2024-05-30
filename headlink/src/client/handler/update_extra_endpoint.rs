use vlink_core::proto::pb::abi::to_client::ToClientData;
use vlink_core::proto::pb::abi::{BcUpdateExtraEndpoint, UpdateExtraEndpoint};
use crate::client::dispatcher::ClientRequest;
use crate::client::error::ExecuteError;
use crate::client::handler::{ExecuteResult, ToServerDataHandler};


///扩展协议启动成功时,客户端向服务器上报该扩展解析的接入端点,每一个客户端只能有一种协议的接入方式
/// 服务器判断是否是默认协议，如果是默认协议则广播给所有节点,
/// 否则查询在线且未建立连接的节点 发送新的端点
/// 如果在线已经建立连接的节点，则判断协议权重，如果权重大于当前连接协议，则替换

/// 处理直接替换peer连接端点

impl ToServerDataHandler for UpdateExtraEndpoint {
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult {
        let network = ctx.network.clone();
        let self_peer = network.peers
            .read_lock().await
            .get(ctx.client_id.pub_key.as_str()).cloned()
            .ok_or(ExecuteError::PeerNotFound)?;

        let data = ToClientData::UpdateExtraEndpoint(BcUpdateExtraEndpoint {
            pub_key: self_peer.pub_key.clone(),
            proto: self.proto.clone(),
            endpoint: self.endpoint.clone(),
        });
        if let Some(e) = self_peer.model.default_proto.as_ref() {
            if e.as_str() == self.proto {
                //network.broadcast_by();
                //network.broadcast(data, vec![]).await;
                return Ok(());
            }
        };


        Ok(())
    }
}