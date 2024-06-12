// #![feature(const_socketaddr)]

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpStream},
    time::Duration,
};
use std::net::SocketAddr;

use anyhow::Result;
use igd::{AddPortError::PortInUse, aio::search_gateway, SearchOptions};
use log::{error, info};
use tap::TapFallible;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

#[derive(Error, Debug)]
pub enum UpnpError {
    #[error("upnp not support ipv6")]
    Ipv6,
    #[error("upnp PortError port:{0}")]
    PortError(u16),
}

pub struct UpnpOptions {
    pub name: String,
    pub inner_port: u16,
    /// upnp 持续时间
    pub duration: u32,
    pub protocol: igd::PortMappingProtocol,
    pub broadcast_address: Option<SocketAddr>,
}

/// 简单外网端口变化，设置upnp 服务
pub struct UpnpService {
    options: UpnpOptions,
    external_port: Mutex<Option<u16>>,
    /// 内部端口
    inner_port: Mutex<u16>,
    pub token: CancellationToken,
}

impl Drop for UpnpService {
    fn drop(&mut self) {
        self.token.cancel();
        info!("upnp service drop");
    }
}

impl UpnpService {
    pub fn new(options: UpnpOptions) -> Self {
        let token = CancellationToken::new();
        Self {
            inner_port: Mutex::new(options.inner_port),
            options,
            external_port: Mutex::new(None),
            token,
        }
    }
    pub async fn set_inner_port(&self, port: u16) {
        *self.inner_port.lock().await = port
    }
    pub async fn set_external_port(&self, port: u16) -> anyhow::Result<()> {
        let mut old = self.external_port.lock().await;
        if old.is_none() || old.unwrap() != port {
            if let Err(e) = self.add_port(port).await {
                error!("upnp error: {:?}", e);
            };
        };
        old.replace(port);
        Ok(())
    }

    pub async fn add_port(&self, port: u16) -> Result<(SocketAddrV4, SocketAddrV4)> {
        info!("upnp add port {}", port);
        let mut options: SearchOptions = Default::default();
        if let Some(addr) = self.options.broadcast_address {
            options.broadcast_address = addr;
        }
        let gateway = search_gateway(options).await?;
        info!("upnp gateway {:?}", gateway);
        let gateway_addr = gateway.addr;
        let stream = TcpStream::connect(gateway_addr)?;
        let addr = stream.local_addr()?;
        //获取网关下的本地ip
        let ip = addr.ip();
        drop(stream);
        // let port = self.options.port;
        let name = self.options.name.as_str();
        let duration = self.options.duration;
        if let IpAddr::V4(ip) = ip {
            let mut retry = true;
            loop {
                //添加端口
                let inner_port = self.inner_port.lock().await.clone();
                if inner_port <= 0 {
                    Err(UpnpError::PortError(inner_port))?;
                }
                return match gateway
                    .add_port(self.options.protocol,
                              port,
                              SocketAddrV4::new(ip, inner_port),
                              duration,
                              name,
                    )
                    .await
                {
                    Err(err) => {
                        if let PortInUse = err {
                            if retry {
                                retry = false;
                                match gateway
                                    .remove_port(igd::PortMappingProtocol::TCP, port)
                                    .await
                                {
                                    Err(err) => {
                                        info!("upnp remove port {} error {}", port, err);
                                    }
                                    Ok(_) => {
                                        continue;
                                    }
                                }
                            }
                        }
                        //info!("upnp {} > {}", gateway_addr, err);
                        Err(err.into())
                    }
                    Ok(_) => {
                        Ok((gateway_addr, SocketAddrV4::new(ip, inner_port)))
                    }
                };
            }
        } else {
            return Err(UpnpError::Ipv6.into());
        }
    }


    pub async fn start(&self) -> anyhow::Result<()> {
        let mut local_ip = Ipv4Addr::UNSPECIFIED;
        let mut pre_gateway = SocketAddrV4::new(local_ip, 0);
        // let sleep_seconds = self.options.sleep_seconds;
        // let duration = sleep_seconds + 60;
        if self.options.duration < 60 {
            return Err(anyhow::anyhow!("upnp duration must > 60"));
        }
        let duration = self.options.duration - 10;
        //结束前10秒上报
        let mut interval = time::interval(Duration::from_secs(duration.into()));
        loop {
            interval.tick().await;
            if self.token.is_cancelled() {
                error!("upnp service cancelled");
                break;
            }
            let _ = timeout(Duration::from_secs(2), async {
                if let Some(port) = self.external_port.lock().await.clone() {
                    match self.add_port(port).await {
                        Ok((gateway, ip)) => {
                            if *ip.ip() != local_ip || gateway != pre_gateway {
                                local_ip = ip.ip().clone();
                                pre_gateway = gateway;
                                info!("upnp add port {} > {}", gateway, ip);
                                // watch.ok(SocketAddrV4::new(ip, port), gateway);
                            }
                        }
                        Err(err) => {
                            local_ip = Ipv4Addr::UNSPECIFIED;
                            error!("upnp error: {:?}", err);
                        }
                    }
                }
            }).await.tap_err(|_| error!("upnp timeout"));
        }
        Ok(())
    }
}


