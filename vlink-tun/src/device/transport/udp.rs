use std::fmt::{Debug, Display, Formatter, Pointer, write};
use std::io;
use std::io::Error;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::sync::Arc;
use async_trait::async_trait;
use socket2::{Domain, Protocol, Type};
use tokio::net::UdpSocket;
use log::{error, info};
use thiserror::__private::AsDisplay;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use crate::device::inbound::{BoxCloneOutboundSender, InboundResult, OutboundSender};
use crate::device::transport::{Endpoint, Transport, TransportDispatcher, TransportInbound, TransportOutbound, TransportWrapper};
pub const PROTO_NAME: &str = "Udp";
/// UdpTransport is a UDP endpoint that implements the [`Transport`] trait.
#[derive(Clone, Debug)]
pub struct UdpTransport {
    port: u16,
    ipv4: Arc<UdpSocket>,
    ipv6: Arc<UdpSocket>,
    ipv4_buf: Vec<u8>,
    ipv6_buf: Vec<u8>,
}

pub struct UdpSocketInfo {
    pub ipv4: Arc<UdpSocket>,
    pub ipv6: Arc<UdpSocket>,
}

#[derive(Clone)]
pub struct UdpOutboundSender {
    pub(crate) dst: SocketAddr,
    pub(crate) ipv4: Arc<UdpSocket>,
    pub(crate) ipv6: Arc<UdpSocket>,
}


impl BoxCloneOutboundSender for UdpOutboundSender {
    fn box_clone(&self) -> Box<dyn OutboundSender> {
        Box::new(self.clone())
    }
}

impl Debug for UdpOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.as_display(), f)
    }
}

impl Display for UdpOutboundSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "UdpEndpoint -> ({})", self.dst)
    }
}

#[async_trait]
impl OutboundSender for UdpOutboundSender {
    async fn send(&self, data: &[u8]) -> Result<(), Error> {
        let dst = self.dst.clone();
        match dst {
            SocketAddr::V4(_) => self.ipv4.send_to(data, dst).await?,
            SocketAddr::V6(_) => self.ipv6.send_to(data, dst).await?,
        };
        Ok(())
    }

    fn dst(&self) -> SocketAddr {
        self.dst.clone()
    }

    fn protocol(&self) -> String {
        PROTO_NAME.to_string()
    }
}


impl UdpTransport {
    pub(crate) async fn spawn(token: CancellationToken, port: u16, sender: Sender<InboundResult>) -> Result<(u16, UdpSocketInfo), io::Error> {
        // tokio::spawn()
        let mut udp = Self::bind(Ipv4Addr::UNSPECIFIED, Ipv6Addr::UNSPECIFIED, port).await?;
        let info = UdpSocketInfo {
            ipv4: udp.ipv4.clone(),
            ipv6: udp.ipv6.clone(),
        };
        //接受数据转到sender 中
        let ipv4c = udp.ipv4.clone();
        let ipv6c = udp.ipv6.clone();
        let inbound = async move {
            loop {
                match udp.recv_from().await {
                    Ok((addr, data)) => {
                        let udp_sender = UdpOutboundSender {
                            dst: addr,
                            ipv4: ipv4c.clone(),
                            ipv6: ipv6c.clone(),
                        };
                        let _ = sender.send((data, Box::new(udp_sender))).await;
                    }
                    Err(e) => {
                        error!("Failed to receive data: {e}");
                        break;
                    }
                }
                // sender.send((data, endpoint)).await.unwrap();
            }
        };
        tokio::spawn(async move {
            loop {
                select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    _ = inbound => {
                        break;
                    }
                }
            }
        });

        Ok((port, info))
    }

    pub(crate) async fn bind(ipv4: Ipv4Addr, ipv6: Ipv6Addr, port: u16) -> Result<Self, io::Error> {
        let (ipv4, ipv6, port) = Self::bind_socket(ipv4, ipv6, port).await?;
        info!(
            "Listening on {} / {}",
            ipv4.local_addr()?,
            ipv6.local_addr()?
        );
        Ok(Self {
            port,
            ipv4,
            ipv6,
            ipv4_buf: vec![],
            ipv6_buf: vec![],
        })
    }
    async fn bind_socket(
        ipv4: Ipv4Addr,
        ipv6: Ipv6Addr,
        port: u16,
    ) -> Result<(Arc<UdpSocket>, Arc<UdpSocket>, u16), io::Error> {
        let max_retry = if port == 0 { 10 } else { 1 };
        let mut err = None;
        for _ in 0..max_retry {
            let ipv4 = match Self::bind_socket_v4(SocketAddrV4::new(ipv4, port)).await {
                Ok(s) => s,
                Err(e) => {
                    err = Some(e);
                    continue;
                }
            };
            let port = ipv4.local_addr()?.port();
            let ipv6 = match Self::bind_socket_v6(SocketAddrV6::new(ipv6, port, 0, 0)).await {
                Ok(s) => s,
                Err(e) => {
                    err = Some(e);
                    continue;
                }
            };

            return Ok((Arc::new(ipv4), Arc::new(ipv6), port));
        }
        let e = err.unwrap();
        error!("Inbound is not able to bind port {port}: {e}");
        Err(e)
    }

    async fn bind_socket_v4(addr: SocketAddrV4) -> Result<UdpSocket, io::Error> {
        let socket = socket2::Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_nonblocking(true)?;
        socket.set_reuse_address(true)?;
        socket.bind(&addr.into())?;
        UdpSocket::from_std(std::net::UdpSocket::from(socket))
    }
    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    async fn bind_socket_v6(addr: SocketAddrV6) -> Result<UdpSocket, io::Error> {
        let socket = socket2::Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_only_v6(true)?;
        socket.set_nonblocking(true)?;
        socket.set_reuse_address(true)?;
        socket.bind(&addr.into())?;
        UdpSocket::from_std(std::net::UdpSocket::from(socket))
    }
    async fn recv_from(&mut self) -> Result<(SocketAddr, Vec<u8>), io::Error> {
        if self.ipv4_buf.is_empty() {
            self.ipv4_buf = vec![0u8; 2048];
        }
        if self.ipv6_buf.is_empty() {
            self.ipv4_buf = vec![0u8; 2048];
        }

        let (data, addr) = tokio::select! {
            ret = self.ipv4.recv_from(&mut self.ipv4_buf) => {
                let (n, addr) = ret?;
                (self.ipv4_buf[..n].to_vec(), addr)
            },
            ret = self.ipv6.recv_from(&mut self.ipv6_buf) => {
                let (n, addr) = ret?;
                (self.ipv6_buf[..n].to_vec(), addr)
            },
        };

        Ok((addr, data))
    }
}

impl Display for UdpTransport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UdpTransport[{}/{}]",
            self.ipv4.local_addr().unwrap(),
            self.ipv6.local_addr().unwrap()
        )
    }
}