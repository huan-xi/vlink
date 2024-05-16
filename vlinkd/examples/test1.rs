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