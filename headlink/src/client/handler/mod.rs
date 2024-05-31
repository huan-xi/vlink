use crate::client::dispatcher::{ClientRequest, RequestContext};
use crate::client::error::ExecuteError;

mod req_config;
mod peer_enter;
mod update_extra_endpoint;
mod dev_handshake_complete;
mod helpers;

pub type ExecuteResult = Result<(), ExecuteError>;

pub trait ToServerDataHandler {
    fn execute(&self, ctx: ClientRequest) -> impl std::future::Future<Output = ExecuteResult> + Send;
}
