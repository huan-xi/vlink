use tokio::sync::broadcast;
use vlink_tun::device::config::{TransportConfig, TransportType};

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<u8>(10);
    let mut rx1 = tx.subscribe();

    tx.send(1).unwrap();
    tx.send(2).unwrap();
    tx.send(3).unwrap();
    tx.send(4).unwrap();
    let mut rx = tx.subscribe();
    while let a = rx.recv().await.unwrap() {
        println!("{}", a)
    }
}
#[cfg(test)]
pub mod test {
    use vlink_tun::device::config::{TransportConfig, TransportType};

    #[test]
    pub fn test() {
        let a = TransportConfig {
            trans_type: TransportType::NatUdp,
            params: "abcd".to_string(),
        };
        println!("{}", serde_json::to_string(&a).unwrap());

    }
}
