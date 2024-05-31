use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Error;
use std::net::SocketAddr;
use async_trait::async_trait;
use tokio::select;
use crate::device::endpoint::Endpoint;
use crate::device::transport::udp::UdpTransport;

/// 标准的udp 协议
pub mod udp;
