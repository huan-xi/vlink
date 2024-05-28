use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("server not connected")]
    ServerNotConnected,
}