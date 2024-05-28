use std::sync::Arc;
use tokio::net::UdpSocket;

/// 公网tcp 传输
/// tcp 传输协议
#[derive(Clone, Debug)]
pub struct UdpTransport {
    port: u16,
    ipv4: Arc<UdpSocket>,
    ipv6: Arc<UdpSocket>,
    ipv4_buf: Vec<u8>,
    ipv6_buf: Vec<u8>,
}