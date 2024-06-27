use tokio::sync::broadcast;

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
    use std::env::set_var;
    use env_logger::Env;
    use log::info;
    use tokio::time;
    use vlink_tun::device::config::TransportConfig;
    use vlinkd::transport::proto::nat_udp;


    pub fn test(c: &mut String) {
        c = "abcd".to_string();
    }


    #[test]
    pub fn test() {
        let a = TransportConfig {
            proto: nat_udp::PROTO_NAME.to_string(),
            params: "abcd".to_string(),
        };
        println!("{}", serde_json::to_string(&a).unwrap());
    }


    #[tokio::test]
    pub async fn test_watch() {
        set_var("RUST_LOG", "debug");
        env_logger::init();
        let (tx, mut rx) = tokio::sync::watch::channel(0);
        let c = tx.clone();


        tokio::spawn(async move {
            while let Ok(a) = rx.changed().await {
                let a = *rx.borrow();
                info!("{:?}", a);
            }
        });
        for i in 0..10 {
            c.send(i).unwrap();
            time::sleep(time::Duration::from_nanos(1)).await;
        }
        time::sleep(time::Duration::from_secs(10)).await;
    }
}
