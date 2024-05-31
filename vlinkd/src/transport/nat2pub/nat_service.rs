use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use log::{debug, error, info, warn};
use stun_format::Attr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use crate::transport::nat2pub::reuse_socket::{make_tcp_socket, make_udp_socket};
use crate::transport::nat2pub::upnp_service::{UpnpOptions, UpnpService};

pub struct NatService {
    /// 本机监听地址
    local_addr: SocketAddr,
    /// upnp 服务
    upnp_service: Arc<UpnpService>,
    /// stun 服务器
    stun_servers: Vec<String>,
    protocol: igd::PortMappingProtocol,
    token: CancellationToken,
}

impl Drop for NatService {
    fn drop(&mut self) {
        self.upnp_service.token.cancel()
    }
}

pub struct NatServiceParam {
    pub stun_servers: Vec<String>,
    /// 本地用户端口复用的udp 端口
    pub port: u16,
    pub(crate) protocol: igd::PortMappingProtocol,
    /// upnp广播地址，默认内网广播
    pub upnp_broadcast_address: Option<SocketAddr>,

}

impl NatService {
    pub fn new(param: NatServiceParam) -> Self {
        let port = param.port;
        let local_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port));
        let options = UpnpOptions {
            name: "vlink".to_string(),
            inner_port: port,
            duration: 140,
            protocol: param.protocol,
            broadcast_address: param.upnp_broadcast_address,
        };
        let token = CancellationToken::new();
        Self {
            local_addr,
            token,
            upnp_service: Arc::new(UpnpService::new(options)),
            stun_servers: param.stun_servers,
            protocol: param.protocol,
        }
    }

    /// 启动nat 服务
    pub async fn start(&self) -> anyhow::Result<mpsc::Receiver<SocketAddrV4>> {
        let mut upnp_service = self.upnp_service.clone();
        let (tx, rx) = mpsc::channel(1);
        //启动upnp 服务
        tokio::spawn(async move {
            if let Err(e) = upnp_service.start().await {
                error!("upnp error: {:?}", e);
            }
        });
        // 与服务器连接，握手，获取远程端口
        let addr = self.local_addr;
        let upnp = self.upnp_service.clone();
        let protocol = self.protocol;
        let token = self.token.clone();
        tokio::spawn(async move {
            let res = stun_connect(protocol, token, addr, upnp, tx).await;
            if let Err(e) = res {
                error!("stun error: {:?}", e);
            };
        });

        Ok(rx)
    }
}


async fn stun_connect(protocol: igd::PortMappingProtocol, token: CancellationToken, addr: SocketAddr, upnp: Arc<UpnpService>, tx: mpsc::Sender<SocketAddrV4>) -> anyhow::Result<()> {
    match protocol {
        igd::PortMappingProtocol::UDP => {
            stun_connect_udp(token, addr, upnp, tx).await
        }
        igd::PortMappingProtocol::TCP => {
            stun_connect_tcp(token, addr, upnp, tx).await
        }
    }
}

async fn stun_connect_tcp(token: CancellationToken, addr: SocketAddr, upnp_service: Arc<UpnpService>, tx: mpsc::Sender<SocketAddrV4>) -> anyhow::Result<()> {
    let time = std::time::Instant::now();
    let socket = make_tcp_socket(addr)?;
    let port=socket.local_addr()?.port();
    upnp_service.set_inner_port(port).await;
    //todo stun 服务器
    let stun_server: SocketAddr = "101.43.169.183:3478".parse().unwrap();
    let mut stream = socket.connect(stun_server).await?;
    info!("Connected to {}", stream.peer_addr().unwrap());
    info!("Local addr: {}", stream.local_addr().unwrap());
    let (mut reader, mut writer) = stream.split();
    let mut buf = [0u8; 28];
    let mut msg = stun_format::MsgBuilder::from(buf.as_mut_slice());
    msg.typ(stun_format::MsgType::BindingRequest).unwrap();
    msg.tid(1).unwrap();
    writer.write(msg.as_bytes()).await?;


    let mut buffer = [0; 1024];
    let mut mapped_addr = None;

    while let Ok(n) = reader.read(&mut buffer).await {
        // let bytes = buffer[..n].to_vec();
        let msg = stun_format::Msg::from(&buffer[..n]);
        for addr in msg.attrs_iter() {
            match addr {
                Attr::MappedAddress(addr) => {
                    debug!("MappedAddress: {:?}", addr);
                    mapped_addr = Some(addr);
                }
                Attr::ChangedAddress(addr) => {
                    debug!("ChangedAddress: {:?}", addr);
                }
                Attr::XorMappedAddress(addr) => {
                    debug!("XorMappedAddress: {:?}", addr);
                    // mapped_addr = Some(addr);
                    match addr {
                        stun_format::SocketAddr::V4(v4, port) => {
                            debug!("pub addr:{}.{}.{}.{}:{}",v4[0],v4[1],v4[2],v4[3],port);
                            tx.send(SocketAddrV4::new(Ipv4Addr::from(v4), port)).await?;
                            upnp_service.add_port(port).await?;
                            // upnp_service.set_external_port(v4.port()).await?;
                        }
                        stun_format::SocketAddr::V6(v6, port) => {
                            warn!("ipv6 not support");
                            todo!("test")
                        }
                    }
                }
                _ => {}
            }
        }
    }
    info!("stun 服务器断开 ,总连接时长{:?}", time.elapsed());

    Ok(())
}


/// 获取公网端口
async fn stun_connect_udp(token: CancellationToken, local_addr: SocketAddr, upnp_service: Arc<UpnpService>, tx: mpsc::Sender<SocketAddrV4>) -> anyhow::Result<()> {
    // let time = std::time::Instant::now();
    let socket = make_udp_socket(local_addr)?;
    info!("stun_connect_udp local_addr:{:?}",socket.local_addr().unwrap());
    upnp_service.set_inner_port(socket.local_addr()?.port()).await;
    let stun_server: SocketAddr = "101.43.169.183:3479".parse().unwrap();
    socket.connect(stun_server).await?;
    socket.send(bind_request().as_slice()).await?;
    let upnp_service = upnp_service.clone();
    //token
    let f = async move {
        if let Err(e) = recv_from_udp(tx, socket, upnp_service).await {
            error!("recv_from_udp error: {:?}", e);
        }
    };
    tokio::spawn(async move {
        loop {
            select! {
                _ = f => {
                    break;
                }
                _ = token.cancelled() => {
                    break;
                }
            }
        }
    });
    Ok(())
}

async fn recv_from_udp(tx: Sender<SocketAddrV4>, socket: UdpSocket, upnp_service: Arc<UpnpService>) -> anyhow::Result<()> {
    let mut buffer = [0; 1024];
    while let Ok(n) = socket.recv(&mut buffer).await {
        let msg = stun_format::Msg::from(&buffer[..n]);
        info!("msg: {:?}", msg);
        for addr in msg.attrs_iter() {
            match addr {
                Attr::MappedAddress(addr) => {
                    debug!("MappedAddress: {:?}", addr);
                }
                Attr::ChangedAddress(addr) => {
                    debug!("ChangedAddress: {:?}", addr);
                }
                Attr::XorMappedAddress(addr) => {
                    let std_addr = match addr {
                        stun_format::SocketAddr::V4(ip, port) => {
                            SocketAddrV4::new(Ipv4Addr::from(ip), port)
                        }
                        stun_format::SocketAddr::V6(_, _) => {
                            todo!("");
                        }
                    };
                    tx.send(std_addr).await?;
                    debug!("XorMappedAddress: {}", std_addr);
                    match addr {
                        stun_format::SocketAddr::V4(ip, port) => {
                            upnp_service.set_external_port(port).await?;
                            let _ = tx.send(SocketAddrV4::new(Ipv4Addr::from(ip), port)).await;
                        }
                        stun_format::SocketAddr::V6(_, _) => {
                            warn!("ipv6 not support");
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn bind_request() -> Vec<u8> {
    let mut buf = [0u8; 28];
    let mut msg = stun_format::MsgBuilder::from(buf.as_mut_slice());
    msg.typ(stun_format::MsgType::BindingRequest);
    msg.tid(1);
    msg.as_bytes().to_vec()
}