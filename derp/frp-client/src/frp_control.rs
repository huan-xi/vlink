use std::sync::Arc;
use std::time::Duration;
use bytes::{Buf, BufMut, BytesMut};
use futures_util::{AsyncWriteExt, FutureExt};
use log::{error, info};
use tokio::time::timeout;
use tokio_util::codec::Framed;
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};
use yamux::{Stream as YamuxStream, Stream};
use crate::conn_control::ConnControl;
use crate::errors::Error;
use crate::frp_codec::{FrpCodec, FrpRequest, FrpResponse};
use crate::msg::{LoginRequest, NewWorkConn, StartWorkConnResp};
use futures_util::{AsyncReadExt, SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use tokio::{io, select};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use crate::msg;
use futures::io::{AsyncRead as FAsyncRead, AsyncWrite as FAsyncWrite};
use tokio::net::TcpStream;

type SinkPtr = Arc<Mutex<SplitSink<Framed<Compat<YamuxStream>, FrpCodec>, FrpRequest>>>;

pub struct FrpControl {
    conn_ctrl: Arc<ConnControl>,
    token: String,
    run_id: Option<String>,
    pub main_sink: SinkPtr,
    pub main_stream: Option<SplitStream<Framed<Compat<YamuxStream>, FrpCodec>>>,
    cancel_token: CancellationToken,
}

#[derive(Clone)]
pub struct RunContext {
    run_id: String,
    token: String,
    pub main_sink: SinkPtr,
    conn_ctrl: Arc<ConnControl>,
}

pub async fn handle_stream(ctx: RunContext, mut stream: SplitStream<Framed<Compat<YamuxStream>, FrpCodec>>) -> Result<(), Error> {
    let sink_ptr = ctx.main_sink.clone();

    while let Some(Ok(msg)) = stream.next().await {
        info!("收到{msg:?}消息");
        match msg {
            FrpResponse::TypeReqWorkConn => {
                //let run_id = ;
                let ws = ctx.conn_ctrl.open_stream().await.unwrap();
                //创建work
                let run_id = ctx.run_id.clone();
                let token = ctx.token.clone();
                tokio::spawn(async move {
                    match hande_work_conn(ws, run_id.as_ref(), token.as_str()).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("处理work_conn失败,错误信息:{:?}", e);
                        }
                    };
                });
            }
            _ => {}
        }
    };
    error!("main_stream is DOWN");

    Ok(())
}

pub async fn proxy<S1, S2>(stream1: S1, stream2: S2) -> io::Result<()>
    where
        S1: AsyncRead + AsyncWrite + Unpin,
        S2: FAsyncRead + FAsyncWrite + Unpin,
{
    let (mut s1_read, mut s1_write) = io::split(stream1);
    let (s2_read, s2_write) = stream2.split();
    let mut s2_read = s2_read.compat();
    let mut s2_write = s2_write.compat_write();
    let res = select! {
        res = io::copy(&mut s1_read, &mut s2_write) => res,
        res = io::copy(&mut s2_read, &mut s1_write) => res,
    };
    res?;
    Ok(())
}

async fn hande_work_conn(mut work_stream: Stream, run_id: &str, token: &str) -> Result<(), Error> {
    info!("创建work_conn {run_id}");
    let work_conn = NewWorkConn::new(run_id, token);

    let mut bytes = BytesMut::new();
    let str = serde_json::to_string(&work_conn)?;
    bytes.put_u8(msg::TYPE_NEW_WORK_CONN.0);
    bytes.put_u64(str.as_bytes().len() as u64);
    bytes.put_slice(str.as_bytes());
    work_stream.write_all(&bytes).await?;

    let mut msg_hdr = [0; msg::MSG_HEADER_SIZE];
    work_stream.read_exact(msg_hdr.as_mut()).await?;
    // let tb = msg_hdr.get_u8();
    let len = u64::from_be_bytes(msg_hdr[1..9].try_into().unwrap());
    let mut data = vec![0; len as usize];
    work_stream.read_exact(&mut data).await?;
    let resp: StartWorkConnResp = serde_json::from_slice(data.as_slice())?;
    info!("work_conn resp {:?}", resp);

    let mut local_stream = TcpStream::connect(format!("{}:{}", "192.168.3.142", 80)).await?;

    proxy(local_stream, work_stream).await?;
    Ok(())
}

impl FrpControl {
    pub(crate) async fn login(&mut self, user: &str) -> Result<(), Error> {
        timeout(Duration::from_secs(1), async {
            let login_msg = LoginRequest::new(self.token.as_str(), user);
            // sink.send(FrpRequest::Login(login_msg))
            self.main_sink.lock().await.send(FrpRequest::Login(login_msg)).await.unwrap();
            let main_stream = self.main_stream
                .as_mut()
                .ok_or(Error::from("main_stream is None"))?;

            if let Some(Ok(FrpResponse::LoginResp(resp))) = main_stream.next().await {
                if let Some(e) = resp.error {
                    return Err(Error::from(format!("登入失败,错误信息:{:?}", e)));
                };
                match resp.run_id.as_ref() {
                    None => {
                        return Err(Error::from("登入失败,未获取到run_id"));
                    }
                    Some(run_id) => {
                        self.run_id.replace(run_id.clone());
                    }
                }
                info!("登入成功,登录信息:{:?}", resp.login_message);
                //等下一个iv 包，秘钥交换
                if let Some(Ok(FrpResponse::IV)) = main_stream.next().await {
                    self.main_sink.lock().await.send(FrpRequest::IV).await?;
                }
            } else {
                return Err(Error::from("未收到登录响应"));
            }
            Ok(())
        }).await.map_err(|e| Error::from("登录响应超时"))?
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let stream = self.main_stream.take();
        match stream {
            None => {
                return Err(Error::from("main_stream is None"));
            }
            Some(stream) => {
                let token = self.cancel_token.clone();
                let context = RunContext {
                    run_id: self.run_id.clone().ok_or(Error::from("run_id is None"))?,
                    token: self.token.clone(),
                    main_sink: self.main_sink.clone(),
                    conn_ctrl: self.conn_ctrl.clone(),
                };
                // let sink_ptr = self.main_sink.clone();
                tokio::spawn(async move {
                    loop {
                        select! {
                            _ = token.cancelled() => {
                                break;
                            }
                            _ = handle_stream(context,stream) =>{
                                break;
                            }
                        }
                    }
                    error!("Device outbound loop is DOWN");
                });
            }
        }
        Ok(())
    }

    pub async fn new(conn_ctrl: ConnControl, token: String, cancel_token: CancellationToken) -> Result<Self, Error> {
        let conn_ctrl = Arc::new(conn_ctrl);
        let mut stream_main = conn_ctrl.open_stream().await?;
        let codec = FrpCodec::new(token.as_str());
        let (main_sink, mut main_stream) = Framed::new(stream_main.compat(), codec).split();

        Ok(Self {
            conn_ctrl,
            token,
            main_sink: Arc::new(Mutex::new(main_sink)),
            main_stream: Some(main_stream),
            run_id: None,
            cancel_token,
        })
    }
}