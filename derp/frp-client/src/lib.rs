mod frp_codec;
mod errors;
mod msg;
mod buf_crypto;
mod service;
mod conn_control;
mod frp_control;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FRP_VERSION: &str = "0.44.0";

pub const PAYLOAD_SIZE: usize = 128 * 1024;


#[cfg(test)]
mod tests1 {
    use std::env;
    use std::sync::Arc;
    use bytes::{BufMut, BytesMut};
    use futures_util::{AsyncReadExt, AsyncWriteExt, future, SinkExt, stream, Stream, StreamExt};
    use futures_util::future::join_all;
    use log::{error, info};
    use tokio::sync::Mutex;
    use tokio::{join, task, time};
    use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
    use tokio_util::sync::CancellationToken;
    use tracing_subscriber::util::SubscriberInitExt;
    use crate::frp_codec::{FrpCodec, FrpRequest, FrpResponse};
    use crate::conn_control::ConnControl;
    use crate::errors::Error;
    use crate::frp_control::FrpControl;
    use crate::msg::{LoginRequest, NewProxyRequest, NewWorkConn, PingRequest};

    pub async fn noop_server(c: impl Stream<Item=Result<yamux::Stream, yamux::ConnectionError>>) {
        c.for_each(|maybe_stream| {
            info!("drop(maybe_stream)");
            drop(maybe_stream);
            future::ready(())
        }).await;
    }


    //ningbo-3689d402.of-7af93c01.shop
    #[tokio::test]
    pub async fn test() -> Result<(), Error> {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
                // std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            ))
            .with(tracing_subscriber::fmt::layer())
            .init();

        // let token = "OpenFrpToken";
        // let addr = "shijiazhuang-5b072fbb.of-7af93c01.shop:8120";
        let token = "";
        let addr = "127.0.0.1:7001";
        // let addr = "ningbo-3689d402.of-7af93c01.shop:8120";
        let mut stream = tokio::net::TcpStream::connect(addr)
            .await.unwrap()
            .compat();
        //多路复用连接
        let conn = yamux::Connection::new(stream, yamux::Config::default(), yamux::Mode::Client);
        let (conn_ctrl, event_loop) = ConnControl::new(conn);
        let cancel_token = CancellationToken::new();
        let cancel_token_c = cancel_token.clone();
        tokio::spawn(async move {
            let res = event_loop.await;
            //发送
            cancel_token_c.cancel();
        });
        // 主控
        let mut frp_ctrl = FrpControl::new(conn_ctrl, token.to_string(), cancel_token).await?;
        // 登入
        frp_ctrl.login("6e1c9d1c57503bc421f61d12ac49949d").await?;
        //发送配置的代理
        //接受指令
        frp_ctrl.run().await?;

        //发起穿透请求
        let mut new_proxy = NewProxyRequest::new("test", "tcp");
        new_proxy.set_remote_port(58986);
        frp_ctrl.main_sink.lock().await.send(FrpRequest::NewProxy(new_proxy)).await.expect("TODO: panic message");

        tokio::signal::ctrl_c().await.unwrap();


        // Ok::<frp_ctrl, Error>(())


        // 发送代理请求

        // let ctrl = Arc::new(ctrl);
        // let ctrl_c = ctrl.clone();


        /*
                                    FrpResponse::TypeReqWorkConn => {
                                        //服务端请求创建 创建work_conn,发送登入时的run_id
                                        let run_id = run_id_lock.lock().await.as_ref().cloned().unwrap();
                                        info!("创建work_conn {run_id}");

                                        let mut ws = conn_ctrl.open_stream().await.unwrap();
                                        let work_conn = NewWorkConn::new(run_id.as_str(), token);

                                        let mut bytes = BytesMut::new();
                                        let str = serde_json::to_string(&work_conn).unwrap();
                                        tracing::info!("send data:{}",str);

                                        bytes.put_u8(msg::TYPE_NEW_WORK_CONN.0);
                                        bytes.put_u64(str.as_bytes().len() as u64);
                                        bytes.put_slice(str.as_bytes());

                                        ws.write_all(&bytes).await.unwrap();

                                        tokio::spawn(async move {
                                            let mut buf = [0; 1024];
                                            while let Ok(n) = ws.read(&mut buf).await {
                                                if n == 0 {
                                                    break;
                                                };
                                                info!("{:?}", n);
                                            }
                                        });

                                        /*
                                        let codec= FrpCodec::new(token);
                                             let (mut work_sink, work_stream) = tokio_util::codec::Framed::new(ws.compat(), codec).split();
                                             work_sink.send(FrpRequest::ReqWorkConn(work_conn)).await.expect("TODO: panic message");
             */
                                        // let ws = s1.lock().await.take().unwrap();
                                        // let run_id_c = run_id_lock.clone();
                                        // let work_conn = NewWorkConn::new(run_id.as_str(), token);
                                        /*  let work_conn = NewWorkConn::from_run_id(run_id.as_str());

                                          let mut bytes = BytesMut::new();
                                          let str = serde_json::to_string(&work_conn).unwrap();
                                          tracing::info!("send data:{}",str);

                                          bytes.put_u8(msg::TYPE_NEW_WORK_CONN.0);
                                          bytes.put_u64(str.as_bytes().len() as u64);
                                          bytes.put_slice(str.as_bytes());

                                          ws.write_all(&bytes).await.unwrap();

                                          tokio::spawn(async move {
                                              let mut buf = [0; 1024];
                                              while let Ok(n) = ws.read(&mut buf).await {
                                                  if n == 0 {
                                                      break;
                                                  };
                                                  info!("{:?}", n);
                                              }
                                          });*/
                                    }
                                    _ => {}
                                }
                            }
                            Some(Err(e)) => {
                                error!("{:?}", e);
                                panic!("{:?}", e);
                                // break;
                            }
                            None => {
                                error!("stream end");
                                break;
                            }
                        }
                    }
                    info!("control is down");
                };*/
        Ok(())
    }

    pub fn test1(a: &mut i32) {
        let a = 1;
        println!("{}", a);
    }

    #[test]
    pub fn test_mut() {
        let mut a = 1;
        test1(&mut a);
        println!("{}", a);
    }
}
