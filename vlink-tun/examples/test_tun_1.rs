use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::process::Command;
use base64::engine::general_purpose::STANDARD as base64Encoding;
use vlink_tun::device::{Device};
use base64::Engine;
use ip_network::{IpNetwork, Ipv4Network};
use serde::Deserialize;
use tokio::fs;
use tokio::io::AsyncReadExt;
use log::info;
use log::log;
use vlink_tun::device::config::{DeviceConfig, PeerConfig};
use vlink_tun::device::peer::cidr::Cidr;

fn decode_base64(s: &str) -> Vec<u8> {
    base64Encoding.decode(s).unwrap()
}

// fn local_private_key() -> [u8; 32] {
//     decode_base64("GDE0rT7tfVGairGhTASn5+ck1mUSqLNyajyMSBFYpVQ=")
//         .try_into()
//         .unwrap()
// }
//
// fn peer_public_key() -> [u8; 32] {
//     decode_base64("ArhPnhqqlroFdP4wca7Yu9PuUR1p+TfMhy9kBewLNjM=")
//         .try_into()
//         .unwrap()
// }


#[derive(Deserialize)]
pub struct Config {
    peer_public_key: String,
    allowed_ips: String,
    peer_endpoint: String,
    private_key: String,
    tun_name: String,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    println!("{args:?}");
    let str = if args.len() == 1 {
        "/Users/huanxi/project/vlink/vlink-tun/examples/config.toml".to_string()
    } else {
        args[1].clone()
    };

    let mut file = fs::File::open(str.as_str()).await?;
    let mut str = String::new();
    file.read_to_string(&mut str).await?;
    let config: Config = toml::from_str(str.as_str()).unwrap();

    info!("Starting");

    let cfg = DeviceConfig {
        private_key: [0u8; 32],
        fwmark: 0,
        port: 0,
        peers: Default::default(),
        address: Ipv4Addr::new(192, 168, 10, 5),
        network: IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(192, 168, 10, 0), 24).unwrap()),
    };
    let cidr = config.allowed_ips.parse::<Cidr>().unwrap();
    let allowed_ips = HashSet::from([cidr]);

    let peer_endpoint: SocketAddr = config.peer_endpoint.parse()?;
    let cfg = cfg.private_key(decode_base64(config.private_key.as_str()).try_into().unwrap());
    let cfg = cfg.peer(PeerConfig {
        public_key: decode_base64(config.peer_public_key.as_str())
            .try_into()
            .unwrap(),
        allowed_ips,
        endpoint: Some(peer_endpoint),
        lazy: false,
        preshared_key: None,
        no_encrypt: false,
        persistent_keepalive: None,
        is_online: false,
        ip_addr: "".to_string(),
    });

    let tun_name = config.tun_name.as_str();
    let device = Device::new(Some(tun_name.to_string()), cfg).await?;

    //设置ip,路由
    // let address = config.address;

    #[cfg(target_os = "macos")]
    {
        let route_add_out = Command::new("sh")
            .arg("-c")
            .arg(format!("ifconfig {tun_name} 172.16.0.2/16 172.16.0.0"))
            .output()
            .expect("sh exec error!");

        let route_add_str: String = format!(
            "route -n add {} -netmask {} -interface {}",
            "172.16.0.0", "255.255.0.0", tun_name
        );
        let route_add_out = Command::new("sh")
            .arg("-c")
            .arg(&route_add_str)
            .output()
            .expect("sh exec error!");
    }
    #[cfg(target_os = "linux")]
    {
        //sudo ifconfig <interface> <ip-address> netmask <netmask>
        let route_add_str: String = format!(
            "ifconfig {tun_name} 172.16.0.3 netmask 255.255.0.0"
        );
        let route_add_out = Command::new("sh")
            .arg("-c")
            .arg(route_add_str.as_str())
            .output()
            .expect("sh exec error!");

        let route_add_str: String = format!(
            "route add -net {} netmask {} dev {}",
            "172.16.0.0/16", "255.255.255.0", tun_name
        );
        let route_add_out = Command::new("sh")
            .arg("-c")
            .arg(route_add_str.as_str())
            .output()
            .expect("sh exec error!");
        log::debug!("linux添加路由,cmd:{}",route_add_str.as_str());
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}

#[cfg(test)]
pub mod test {
    use std::iter::once;

    #[test]
    pub fn test() {
        let str = "hello";
        let a: Vec<u16> = str.encode_utf16().chain(once(0)).collect();
        //[104, 101, 108, 108, 111]
        // let a: Vec<u16> = str.encode_utf16().collect();
        println!("{:?}", a);
    }
}