use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use futures_util::StreamExt;
use igd::PortMappingProtocol;
use log::{debug, error, info, warn};
use stun_format::Attr;
use tokio::sync::mpsc;
use crate::transport::nat2pub::reuse_socket::{make_udp_socket};
use crate::transport::nat2pub::upnp_service::{UpnpOptions, UpnpService};

pub struct NatService {
    /// 本机监听地址
    local_addr: SocketAddr,
    /// upnp 服务
    upnp_service: Arc<UpnpService>,
    /// stun 服务器
    stun_servers: Vec<String>,
    protocol: igd::PortMappingProtocol,
    // nat_conn: Arc<NatConn>,
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
        Self {
            local_addr,
            upnp_service: Arc::new(UpnpService::new(options)),
            stun_servers: param.stun_servers,
            protocol: param.protocol,
        }
    }

    /// 启动nat 服务
    pub async fn start(&self) -> anyhow::Result<Option<stun_format::SocketAddr>> {
        let mut upnp_service = self.upnp_service.clone();
        let (tx, rx) = mpsc::channel(1);

        tokio::spawn(async move {
            if let Err(e) = upnp_service.start().await {
                error!("upnp error: {:?}", e);
            }
        });
        // 与服务器连接，握手，获取远程端口
        Ok(match self.protocol {
            PortMappingProtocol::TCP => {
                //self.stun_connect(self.upnp_service.clone()).await?;
                todo!();
            }
            PortMappingProtocol::UDP => {
                self.stun_connect_udp(tx).await?
            }
        })
    }

    /// 获取公网端口
    async fn stun_connect_udp(&self, tx: mpsc::Sender<SocketAddrV4>) -> anyhow::Result<Option<stun_format::SocketAddr>> {
        let time = std::time::Instant::now();
        let socket = make_udp_socket(self.local_addr)?;
        info!("socket.local_addr():{:?}",socket.local_addr().unwrap());
        self.upnp_service.set_inner_port(socket.local_addr()?.port()).await;
        let stun_server: SocketAddr = "101.43.169.183:3479".parse().unwrap();
        socket.connect(stun_server).await?;
        socket.send(bind_request().as_slice()).await?;
        let mut buffer = [0; 1024];
        let mut mapped_addr = None;
        let upnp_service = self.upnp_service.clone();
        while let Ok(n) = socket.recv(&mut buffer).await {
            let msg = stun_format::Msg::from(&buffer[..n]);
            info!("msg: {:?}", msg);
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
                        mapped_addr = Some(addr);
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
        Ok(mapped_addr)
    }
    async fn stun_connect(&self, upnp_service: Arc<UpnpService>) -> anyhow::Result<Option<stun_format::SocketAddr>> {
        let time = std::time::Instant::now();
        // let socket = make_socket(self.local_addr, self.protocol)?;
        //todo stun 服务器
        let stun_server: SocketAddr = "101.43.169.183:3478".parse().unwrap();
        // let stun_server = SockAddr::from(stun_server);
        // let mut stream = socket.connect(stun_server).await?;
        todo!();
        // info!("Connected to {}", stream.peer_addr().unwrap());
        // info!("Local addr: {}", stream.local_addr().unwrap());
        /*
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
                                mapped_addr = Some(addr);
                                match addr {
                                    SocketAddr::V4(v4) => {
                                        upnp_service.set_external_port(v4.port()).await?;
                                    }
                                    SocketAddr::V6(v6) => {
                                        warn!("ipv6 not support");
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                info!("stun 服务器断开 ,总连接时长{:?}", time.elapsed());

                Ok(mapped_addr)*/
    }
}

fn bind_request() -> Vec<u8> {
    let mut buf = [0u8; 28];
    let mut msg = stun_format::MsgBuilder::from(buf.as_mut_slice());
    msg.typ(stun_format::MsgType::BindingRequest);
    msg.tid(1);
    msg.as_bytes().to_vec()
}