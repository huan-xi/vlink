use std::net::Ipv4Addr;
use std::time::Duration;
use libc::sleep;
use vlink_tun::{NativeTun, Tun};
use vlink_tun::tun::IFace;

#[tokio::main]
async fn main() {
    let a = NativeTun::new(Some("vlink0".to_string())).unwrap();

    println!("a:{}", a.name());
    let a = a.set_ip(Ipv4Addr::new(192, 168, 10, 22), Ipv4Addr::new(255, 255, 255, 0));
    println!("{:?}", a);
    /*println!("name:{:?}", a.name());
    a.set_mtu(1499).unwrap();
    let m = a.mtu();
    println!("get_mtu:{:?}", m);
    // tokio::time::sleep(Duration::from_secs(10)).await;
    let res = a.set_address(Ipv4Addr::new(192, 168, 10, 22));
    a.set_netmask(Ipv4Addr::new(255, 255, 255, 0)).unwrap();
    println!("set addr:{:?}", res);
    let addr = a.address();
    println!("get addr:{:?}", addr);*/
    tokio::time::sleep(Duration::from_secs(100)).await;

}

#[cfg(test)]
pub mod test{
    use std::net::Ipv4Addr;
    use ip_network::Ipv4Network;
    use vlink_tun::router::helpers;

    #[test]
    pub fn test() {
        let a = Ipv4Addr::new(255, 255, 255, 0);
        let a = helpers::bite_mask(16);
        let a = Ipv4Addr::from(a);
        print!("{:?}", a);
    /*    let net = Ipv4Network::from_str_truncate("192.168.0.0/24").unwrap();
        print!("{:?}", net);
        let net=Ipv4Network::new(Ipv4Addr::new(192, 168, 10, 0), 24).unwrap();
        print!("{:?}", net);
*/
        println!("test");
    }
}