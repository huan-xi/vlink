use std::str::FromStr;
use native_tls::HandshakeError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use crate::derp_codec::DerpRequest;

#[derive(Debug, Error)]
pub enum Error {
    #[error("MaxPacket")]
    MaxPacket,
    #[error("IoError {0}")]
    IoError(#[from] std::io::Error),
    // native_tls::Error
    #[error("TlsError {0}")]
    TlsError(#[from] native_tls::Error),

    #[error("Error {0}")]
    BoxError(#[from] Box<dyn std::error::Error + Send + Sync>),
    //HandshakeError<std::net::TcpStream>
    #[error("WsHandshakeError {0}")]
    WsHandshakeError(#[from] HandshakeError<std::net::TcpStream>),
    #[error("Error {0}")]
    ErrorMsg(String),
    // httparse::Error
    #[error("HttpError {0}")]
    HttpError(#[from] httparse::Error),
    #[error("MaxFrameLength {0}")]
    MaxFrameLength(usize),
    #[error("InvalidServerKey")]
    InvalidServerKey,
    #[error("InvalidKey")]
    InvalidKey,
    #[error("EncodeError")]
    EncodeError,
    #[error("NoServerKey")]
    NoServerKey,
    #[error("EncryptError")]
    EncryptError,
    #[error("DecryptError")]
    DecryptError,
    #[error("NotConnect")]
    NotConnect,
    #[error("SendError {0:?}")]
    SendError(#[from] SendError<DerpRequest>),
    #[error("HandshakeError")]
    HandshakeError
}

impl Error {}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::ErrorMsg(value)
    }
}

impl FromStr for Error {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Error::ErrorMsg(s.to_string()))
    }
}