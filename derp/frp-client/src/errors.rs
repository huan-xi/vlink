use thiserror::Error;
use crate::conn_control::ControlCommand;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error:{0}")]
    IoError(#[from] std::io::Error),
    //serde_json::Error
    #[error("serde_json error:{0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("ConnectionError: {0}")]
    ConnectionError(#[from]  yamux::ConnectionError),
    #[error("InvalidLength: {0}")]
    Encrypt(String),
    #[error("LoginError msg: {0}")]
    LoginError(String),
    #[error("Error: {0}")]
    ErrorMsg(String),
    #[error("HandlerMsg: {0}")]
    HandlerMsg(String),
    #[error("FrpCodecError: {0}")]
    FrpCodecError(String),
    // futures::futures_channel::mpsc::SendError
    #[error("SendError: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<ControlCommand>),
    //Canceled
    #[error("Canceled: {0}")]
    Canceled(#[from]  tokio::sync::oneshot::error::RecvError),
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::ErrorMsg(value)
    }
}
impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error::ErrorMsg(value.to_string())
    }
}


