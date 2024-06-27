use std::time::Duration;
use tokio::select;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(async move {
        loop {
            // let msg = rx.recv().await;
            // println!("Received: {:?}", msg);
            select! {
                msg=rx.recv()=>println!("Received: {:?}", msg),
            }
        }
    });
    loop {
        tx.send("hello").await.expect("send error");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}