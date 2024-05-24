use axum::Form;
use log::error;
use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecuteError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    StrMessage(&'static str),
    #[error("{0}")]
    DbError(String),
    #[error("{0}")]
    AnyhowError(#[from] anyhow::Error),

    #[error("{0}")]
    ParseError(String),
    #[error("PeerNotFound")]
    PeerNotFound,
    #[error("IpNotMatch")]
    IpNotMatch,
    #[error("IpNotFound")]
    IpNotFound,
}

impl ExecuteError {
    pub fn code(&self) -> i32 {

        match self {
            // -1 代表错误提示
            _ => -1
        }
    }
}

impl From<std::net::AddrParseError> for ExecuteError {
    fn from(value: std::net::AddrParseError) -> Self {
        error!("AddrParseError: {:?}", value);
        ExecuteError::ParseError(value.to_string())
    }
}

impl From<DbErr> for ExecuteError {
    fn from(value: DbErr) -> Self {
        error!("DbErr: {:?}", value);
        ExecuteError::DbError(value.to_string())
    }
}
