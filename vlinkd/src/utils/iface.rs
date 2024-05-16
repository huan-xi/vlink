use pnet::datalink::NetworkInterface;
use pnet::ipnetwork::IpNetwork;

pub fn find_my_ip() -> Option<String> {
    if let Some(e) = find_iface() {
        for ip in e.ips.iter() {
            match ip {
                IpNetwork::V4(i) => {
                    return Some(i.ip().to_string());
                }
                IpNetwork::V6(v6) => {
                    let ip = v6.ip().to_string();
                    if ip.as_str().starts_with("fe80") {
                        continue;
                    }
                    return Some(ip);
                }
            }
        }
    }

    None
}

pub fn find_iface() -> Option<NetworkInterface> {
    let interfaces = pnet::datalink::interfaces();
    interfaces
        .into_iter()
        .filter(|i| i.is_up())
        .filter(|i| i.mac.is_some())
        .filter(|i| !i.name.starts_with("lo") && !i.name.contains("tun") && !i.name.starts_with("anpi"))
        //有ipv4地址
        //&& iface.ips.iter().find(|ip| IpNetwork::is_ipv4(ip)) != None)
        .find(|iface| iface.ips.len() > 0 && filter_only_local_ip(&iface.ips))
}

//查找本地网络
pub fn filter_only_local_ip(ips: &Vec<IpNetwork>) -> bool {
    for ip in ips {
        match ip {
            IpNetwork::V4(v4) => {
                if !v4.to_string().starts_with("127") {
                    return true;
                }
            }
            IpNetwork::V6(v6) => {
                if !v6.to_string().starts_with("fe80") {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
pub mod test {
    use crate::utils::iface::find_my_ip;

    #[test]
    pub fn test() {
        let a = find_my_ip();
        println!("{:?}", a);
    }
}