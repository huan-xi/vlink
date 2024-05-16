use crypto_box::{KEY_SIZE, SecretKey};
use futures_util::{SinkExt, StreamExt};
use log::info;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Framed};
use url::Url;
use crate::derp_codec::{ClientInfo, DerpCodec, DerpRequest, DerpResponse};
use crate::errors::Error;
use crate::ssl::{AsyncConnector, AsyncMaybeTlsStream};

pub struct DerpClient {
    key: [u8; 32],
    server_url: Url,
}

pub type AsyncClient<S> = Framed<S, DerpCodec>;

impl DerpClient {
    pub async fn new(key: [u8; 32], url: &str) -> anyhow::Result<Self> {
        // let mut derp = DerpWebsocketBuilder::new(&url)?;
        // derp.add_header(FAST_START_HEADER.to_string(), "1".to_string());
        // let mut ws_stream = derp
        //     .async_connect().await
        //     .map_err(|e| anyhow::anyhow!("{e}"))?;
        // let (sink, stream) = ws_stream.split::<Message>();
        let url = Url::parse(url)?;


        Ok(Self {
            key,
            server_url: url,
        })
    }

    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }
    pub fn send(&self, dst: &[u8], data: &[u8]) -> Result<(), Error> {
        // self.sink.send()
        if data.len() > super::MAX_PACKET_SIZE {
            return Err(Error::MaxPacket);
        };
        //self.sink.send()

        //todo 限流
        Ok(())
    }


    pub async fn async_connect(&mut self) -> Result<AsyncClient<AsyncMaybeTlsStream>, Error> {
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
        let alice_secret_key = SecretKey::from([1u8; KEY_SIZE]);
        let alice_public_key_bytes = alice_secret_key.public_key().as_bytes().clone();
        let framed = Framed::new(stream, DerpCodec::new(alice_public_key_bytes,
                                                        alice_secret_key.to_bytes()
                                                        , false));
        let (mut sink, mut stream) = framed.split();

        while let Some(e) = stream.next().await {
            match e {
                Ok(n) => {
                    match n {
                        DerpResponse::Ws => {}
                        DerpResponse::FrameServerKey(key) => {
                            //发送客户端key
                            sink.send(DerpRequest::ClientInfo(ClientInfo::new(false))).await?;
                        }
                        _ => {

                        }
                    }
                    info!("test:{n:?}");
                }
                Err(e) => {
                    info!("test:{e:?}");
                }
            }
        };


        todo!();

        // async_connect_on(stream).await
    }
}

#[cfg(test)]
pub mod test {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    #[tokio::test]
    async fn test_derp_client() {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(
                // std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            ))
            .with(tracing_subscriber::fmt::layer())
            .init();


        let key = [0u8; 32];
        let url = "wss://derp6.tailscale.com/derp";
        // let url = "wss://oaodev.local.hperfect.cn:11443/derp";
        let mut derp = super::DerpClient::new(key, url).await.unwrap();
        derp.async_connect().await.unwrap();

        tokio::signal::ctrl_c().await.unwrap();
        assert_eq!(derp.key(), &key);
    }
}