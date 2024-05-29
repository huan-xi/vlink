use crate::client::dispatcher::{ClientRequest, RequestContext};
use crate::client::error::ExecuteError;

mod req_config;
mod peer_enter;
mod update_extra_endpoint;

pub type ExecuteResult = Result<(), ExecuteError>;

pub trait ToServerDataHandler {
    async fn execute(&self, ctx: ClientRequest) -> ExecuteResult;
}