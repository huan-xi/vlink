use bytes::{Buf, BufMut, BytesMut};
use log::{debug, error};
use tokio_util::codec::{Decoder, Encoder};
use log::info;

use crate::buf_crypto::FrpBufCrypto;
use crate::errors::Error;
use crate::frp_codec::Stage::Work;
use crate::msg;
use crate::msg::{LoginRequest, LoginResp, MsgType, NewProxyRequest, NewWorkConn, PingRequest, StartWorkConnResp};

pub enum FrpRequest {
    Login(LoginRequest),
    ReqWorkConn(NewWorkConn),
    Ping(PingRequest),
    NewProxy(NewProxyRequest),
    /// 登入成功后的iv 响应
    IV,
}

#[derive(Debug)]
pub enum FrpResponse {
    LoginResp(LoginResp),
    IV,
    TypeReqWorkConn,
    StartWorkConn(StartWorkConnResp),
    Test,
}

#[derive(Debug)]
pub struct Header {
    type_byte: MsgType,
    len: u64,
}

#[derive(PartialEq)]
pub enum Stage {
    Login,
    WaitIv,
    Work,
}

impl FrpRequest {
    pub(crate) fn is_encrypt(&self) -> bool {
        match self {
            FrpRequest::NewProxy(_) => { true }
            _ => { false }
        }
    }
}

pub struct FrpCodec {
    token: String,
    header: Option<Header>,
    frp_coder: Option<FrpBufCrypto>,
    stage: Stage,
}

impl FrpCodec {
    pub fn new(token: &str) -> Self {
        FrpCodec {
            token: token.to_string(),
            header: None,
            frp_coder: None,
            stage: Stage::Login,
        }
    }
}

/// login
/// iv(16)+type_byte(1)+len(8)+data
impl Decoder for FrpCodec {
    type Item = FrpResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        info!("decode len:{} bytes:{:?}",src.len(),hex::encode(src.to_vec()));
        // info!("decode:{:?}",String::from_utf8_lossy(src.iter().as_slice()));
        if self.stage == Stage::WaitIv {
            if src.len() < 16 {
                return Ok(None);
            }
            let iv = src.split_to(16);
            self.frp_coder = Some(FrpBufCrypto::new(self.token.as_str(), iv.as_ref())?);
            self.stage = Work;
            return Ok(Some(FrpResponse::IV));
        };

        let header = match self.header.take() {
            None => {
                if src.len() < 9 {
                    return Ok(None);
                };
                let mut data = src.split_to(9);
                if let Some(coder) = self.frp_coder.as_mut() {
                    info!("before decrypt_buf:{:?}",data.iter().as_slice());
                    coder.decrypt_buf(data.as_mut());
                    info!("after decrypt_buf:{:?}",data.iter().as_slice());
                }

                let type_byte = data.get_u8();
                let len = data.get_u64();
                Header {
                    type_byte: MsgType(type_byte),
                    len,
                }
            }
            Some(s) => { s }
        };


        let len = header.len as usize;
        //header.type_byte == msg::TYPE_LOGIN && src.len() < len + 16
        if len > 2 << 10 {
            error!("msg len too long:{}", len);
            return Err(Error::FrpCodecError("msg len too long".to_string()));
        };
        // || !is_enough(header.type_byte, src.len(), len)
        if src.len() < len {
            self.header = Some(header);
            return Ok(None);
        }
        let mut data = src.split_to(len);
        //必须要解密数据,否则pos 会移位
        if let Some(c) = self.frp_coder.as_mut() {
            c.decrypt_buf(data.as_mut());
        };
        debug!("recv data:{:?}",String::from_utf8_lossy(data.as_ref()));
        match header.type_byte {
            msg::TYPE_LOGIN_RESP => {
                let resp: LoginResp = serde_json::from_slice(data.iter().as_slice())?;
                self.stage = Stage::WaitIv;
                return Ok(Some(FrpResponse::LoginResp(resp)));
            }
            msg::TYPE_REQ_WORK_CONN => {
                return Ok(Some(FrpResponse::TypeReqWorkConn));
            }
            msg::TYPE_NEW_PROXY_RESP => {
                return Ok(Some(FrpResponse::Test));
            }
            msg::TYPE_START_WORK_CONN=>{
                let data: StartWorkConnResp = serde_json::from_slice(data.iter().as_slice())?;
                return Ok(Some(FrpResponse::StartWorkConn(data)));
            }
            _ => {
                error!("not support type_byte:{}",String::from_utf8_lossy(&[header.type_byte.0]));
                panic!("not support type_byte:{:?}", header.type_byte);
            }
        }
        return Ok(None);
    }
}

fn is_enough(tp: MsgType, src_len: usize, header_len: usize) -> bool {
    match tp {
        //需要额外的iv长度
        msg::TYPE_LOGIN_RESP => src_len >= header_len + 16,
        _ => true,
    }
}

impl Encoder<FrpRequest> for FrpCodec {
    type Error = Error;

    fn encode(&mut self, item: FrpRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut data = BytesMut::new();
        let is_encrypt = item.is_encrypt();
        /*
           if is_encrypt {
               data.put_slice(self.frp_coder.as_ref()
                   .ok_or(Error::FrpCodecError("frp_coder is None".to_string()))?
                   .iv.as_slice());
           };*/

        let str = match item {
            FrpRequest::Login(login) => {
                data.put_u8(msg::TYPE_LOGIN.0);
                serde_json::to_string(&login)?
            }
            FrpRequest::ReqWorkConn(req) => {
                data.put_u8(msg::TYPE_NEW_WORK_CONN.0);
                serde_json::to_string(&req)?
            }
            FrpRequest::Ping(req) => {
                data.put_u8(msg::TYPE_PING.0);
                serde_json::to_string(&req)?
            }
            FrpRequest::NewProxy(req) => {
                data.put_u8(msg::TYPE_NEW_PROXY.0);
                serde_json::to_string(&req)?
            }
            FrpRequest::IV => {
                dst.put_slice(self.frp_coder.as_ref()
                    .ok_or(Error::FrpCodecError("frp_coder is None".to_string()))?
                    .iv.as_slice());
                return Ok(());
            }
            _ => {
                todo!("not implemented");
            }
        };
        let str_bytes = str.as_bytes();
        data.put_u64(str_bytes.len() as u64);
        data.put_slice(str_bytes);

        if is_encrypt {
            self.frp_coder.as_mut()
                .ok_or(Error::FrpCodecError("frp_coder is None".to_string()))?
                .encrypt_buf(data.as_mut());
        }

        dst.put(data);
        Ok(())
    }
}