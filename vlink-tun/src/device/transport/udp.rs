use std::fmt::{Display, Formatter};
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::sync::Arc;
use async_trait::async_trait;
use socket2::{Domain, Protocol, Type};
use tokio::net::UdpSocket;
use log::{error, info};
use crate::device::transport::{Endpoint, Transport, TransportDispatcher};

/// UdpTransport is a UDP endpoint that implements the [`Transport`] trait.
#[derive(Clone,Debug)]

pub struct UdpTransport {
    port: u16,
    ipv4: Arc<UdpSocket>,
    ipv6: Arc<UdpSocket>,
    ipv4_buf: Vec<u8>,
    ipv6_buf: Vec<u8>,
}

impl UdpTransport {

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

    async fn bind_socket_v6(addr: SocketAddrV6) -> Result<UdpSocket, io::Error> {
        let socket = socket2::Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_only_v6(true)?;
        socket.set_nonblocking(true)?;
        socket.set_reuse_address(true)?;
        socket.bind(&addr.into())?;
        UdpSocket::from_std(std::net::UdpSocket::from(socket))
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

#[async_trait]
impl Transport for UdpTransport {
    fn port(&self) -> u16 {
        self.port
    }


    async fn send_to(&self, data: &[u8], dst: SocketAddr,) -> Result<(), io::Error> {
        match dst {
            SocketAddr::V4(_) => self.ipv4.send_to(data, dst).await?,
            SocketAddr::V6(_) => self.ipv6.send_to(data, dst).await?,
        };
        Ok(())
    }

    async fn recv_from(&mut self) -> Result<(Endpoint, Vec<u8>), io::Error> {
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

        Ok((Endpoint::new(TransportDispatcher::Udp(self.clone()), addr), data))
    }

    fn ipv4(&self) -> Ipv4Addr {
        if let SocketAddr::V4(addr) = self.ipv4.local_addr().unwrap() {
            *addr.ip()
        } else {
            unreachable!()
        }
    }

    fn ipv6(&self) -> Ipv6Addr {
        if let SocketAddr::V6(addr) = self.ipv6.local_addr().unwrap() {
            *addr.ip()
        } else {
            unreachable!()
        }
    }
}
