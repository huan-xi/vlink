use crypto_box::{KEY_SIZE, SecretKey};
use futures_util::{SinkExt, Stream, StreamExt};
use futures_util::stream::SplitStream;
use log::info;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_util::codec::{Decoder, Framed};
use url::Url;
use crate::derp_codec::{ClientInfo, DerpCodec, DerpRequest, DerpResponse};
use crate::errors::Error;
use crate::ssl::{AsyncConnector, AsyncMaybeTlsStream};


pub struct DerpClient {
    key: [u8; 32],
    server_url: Url,
    rx: Option<mpsc::Receiver<DerpRequest>>,
    pub sender: mpsc::Sender<DerpRequest>,
}

pub type AsyncClient<S> = Framed<S, DerpCodec>;

impl DerpClient {
    pub async fn new(key: [u8; 32], url: &str) -> anyhow::Result<Self> {
        let url = Url::parse(url)?;
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        Ok(Self {
            key,
            server_url: url,
            sender: tx,
            rx: Some(rx),
        })
    }

    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    // It is an error if the packet is larger than 64KB.
    pub async fn send(&self, dst: &[u8], data: &[u8]) -> Result<(), Error> {
        send_to(&self.sender, dst, data).await
    }


    pub async fn async_connect(&mut self) -> Result<SplitStream<Framed<AsyncMaybeTlsStream, DerpCodec>>, Error> {
        let url = &self.server_url;
        let addr = crate::derp_utils::resolve(url)?;
        let stream = TcpStream::connect(&addr).await?;

        let connector = if url.scheme() == "wss" {
            AsyncConnector::new_with_default_tls_config()?
        } else {
            AsyncConnector::Plain
        };

        let domain = url.domain().unwrap_or("");
        let mut stream = connector.wrap(domain, stream).await?;

        //发送ws upgrade请求
        let headers = vec![];
        let request = crate::derp_utils::build_request(url, &headers);
        AsyncWriteExt::write_all(&mut stream, request.as_bytes()).await?;
        let secret_key = SecretKey::from(self.key);

        let public_key_bytes = secret_key.public_key().as_bytes().clone();
        // info!("alice_public_key_bytes:{:?}", public_key_bytes);
        let framed = Framed::new(stream, DerpCodec::new(public_key_bytes,
                                                        secret_key.to_bytes()
                                                        , false));
        let (mut sink, mut stream) = framed.split();

        let mut rx = self.rx.take().unwrap();
        tokio::spawn(async move {
            while let Some(e) = rx.recv().await {
                if let Err(e) = sink.send(e).await {
                    info!("send error:{:?}",e);
                    break;
                }
            }
            //todo 处理断开事件
        });
        let txc = self.sender.clone();
        let mut stream = await_handshake(stream, txc).await?;

        /* tokio::spawn(async move {
             while let Some(Ok(e)) = stream.next().await {
                 info!("recv data:{:?}", e);
                 match e {
                     DerpResponse::FrameRecvPacket((src, data)) => {
                         info!("recv from {src:?} data:{}", String::from_utf8_lossy(data.as_slice()));
                     }
                     _ => {}
                 }
             }
         });*/
        Ok(stream)
        // async_connect_on(stream).await
    }
}

pub async fn send_to(sender: &mpsc::Sender<DerpRequest>, dst: &[u8], data: &[u8]) -> Result<(), Error> {
    // self.sink.send()
    if data.len() > super::MAX_PACKET_SIZE {
        return Err(Error::MaxPacket);
    };
    //todo 限流
    sender
        .send(DerpRequest::SendPacket((dst.to_vec(), data.to_vec())))
        .await?;
    //self.sink.send()
    Ok(())
}

async fn await_handshake(mut stream: SplitStream<Framed<AsyncMaybeTlsStream, DerpCodec>>, tx: mpsc::Sender<DerpRequest>) -> Result<SplitStream<Framed<AsyncMaybeTlsStream, DerpCodec>>, Error> {
    while let Some(e) = stream.next().await {
        match e {
            Ok(n) => {
                match n {
                    DerpResponse::Ws => {
                        //ws升级成功
                    }
                    DerpResponse::FrameServerKey(key) => {
                        //发送客户端key
                        let _ = tx.send(DerpRequest::ClientInfo(ClientInfo::new(false))).await;
                    }
                    DerpResponse::ServerInfo(info) => {
                        return Ok(stream);
                    }
                    _ => {}
                }
                info!("test:{n:?}");
            }
            Err(e) => {
                return Err(Error::HandshakeError);
            }
        }
    };
    Ok(stream)
}

#[cfg(test)]
pub mod test {
    use std::time::Duration;
    use futures_util::StreamExt;
    use tracing::info;
    use tracing_subscriber::fmt::time;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use crate::derp_codec::DerpResponse;

    #[tokio::test]
    async fn test_derp_client2() {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(
                // std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            ))
            .with(tracing_subscriber::fmt::layer())
            .init();


        let key = [1u8; 32];
        // let url = "wss://derp6.tailscale.com/derp";
        let url = "wss://oaodev.local.hperfect.cn:11443/derp";
        let mut derp = super::DerpClient::new(key, url).await.unwrap();

        derp.async_connect().await.unwrap();
        // let target = [0u8; 32];
        // derp.send(target.as_slice(), b"hello").await.unwrap();

        tokio::signal::ctrl_c().await.unwrap();
        assert_eq!(derp.key(), &key);
    }

    #[tokio::test]
    async fn test_derp_client() {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
                // std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            ))
            .with(tracing_subscriber::fmt::layer())
            .init();


        let key = [0u8; 32];
        // let url = "wss://derp6.tailscale.com/derp";
        let url = "wss://oaodev.local.hperfect.cn:11443/derp";
        let mut derp = super::DerpClient::new(key, url).await.unwrap();

        let mut stream = derp.async_connect().await.unwrap();

        tokio::spawn(async move {
            while let Some(Ok(e)) = stream.next().await {
                info!("recv data:{:?}", e);
                match e {
                    DerpResponse::FrameRecvPacket((src, data)) => {
                        info!("recv from {src:?} data:{}", String::from_utf8_lossy(data.as_slice()));
                    }
                    _ => {}
                }
            }
        });

        let target = [164, 224, 146, 146, 182, 81, 194, 120, 185, 119, 44, 86, 159, 95, 169, 187, 19, 217, 6, 180, 106, 182, 140, 157, 249, 220, 43, 68, 9, 248, 162, 9];
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                info!("send");
                derp.send(target.as_slice(), b"hello").await.unwrap();
                interval.tick().await;
            }
        });

        tokio::signal::ctrl_c().await.unwrap();
        // assert_eq!(derp.key(), &key);
    }
}