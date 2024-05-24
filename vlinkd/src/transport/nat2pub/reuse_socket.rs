use std::io;
use std::net::SocketAddr;
use socket2::{Domain, Socket};
use tokio::net::{TcpSocket, UdpSocket};

pub fn make_tcp_socket(local_addr: SocketAddr) -> anyhow::Result<TcpSocket> {
    let socket = TcpSocket::new_v4()?;
    // allow to reuse the addr both for connect and listen
    #[cfg(target_family = "unix")]
    { socket.set_reuseport(true)?; }
    socket.set_reuseaddr(true)?;
    socket.bind(local_addr).unwrap();
    Ok(socket)
}


pub fn make_udp_socket(local_addr: SocketAddr) -> anyhow::Result<UdpSocket> {
    let addr: socket2::SockAddr = local_addr.into();
    let socket = Socket::new(Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    #[cfg(not(windows))]
    #[cfg(not(target_os = "illumos"))]
    socket.set_reuse_port(true)?;
    socket.bind(&addr)?;
    let udp: std::net::UdpSocket = socket.into();
    Ok(UdpSocket::from_std(udp)?)
}



/*                   /// 创建一个端口复用socket
pub fn make_socket(local_addr: SocketAddr, protocol: igd::PortMappingProtocol) -> anyhow::Result<TokioSocket> {
    Ok(match protocol {
        PortMappingProtocol::TCP => {
            let addr: socket2::SockAddr = local_addr.into();
            let socket = Socket::new(Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
            socket.set_reuse_address(true)?;
            socket.set_nonblocking(true)?;
            #[cfg(not(windows))]
            #[cfg(not(target_os = "illumos"))]
            socket.set_reuse_port(true)?;
            socket.bind(&addr)?;
            let tcp: std::net::TcpStream = socket.into();
            TokioSocket::Tcp(TcpSocket::from_std_stream(tcp))
        }
        PortMappingProtocol::UDP => {
            let addr: socket2::SockAddr = local_addr.into();
            let socket = Socket::new(Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
            socket.set_reuse_address(true)?;
            socket.set_nonblocking(true)?;
            #[cfg(not(windows))]
            #[cfg(not(target_os = "illumos"))]
            socket.set_reuse_port(true)?;
            socket.bind(&addr)?;
            socket
        }
    })
}
*/