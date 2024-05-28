pub mod network;
pub mod client;
pub mod storage;
pub mod connect;
pub mod utils;
pub mod config;
pub mod transport;
pub mod forward;
pub mod error;
mod handler;
pub mod api;

pub const DEFAULT_PORT: u16 = 51820;

use base64::engine::general_purpose::STANDARD as base64Encoding;
