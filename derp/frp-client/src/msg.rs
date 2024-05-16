use chrono::Utc;
use futures_util::io::{AsyncReadExt, AsyncWriteExt};
use md5;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env::consts, mem::size_of};
use log::debug;
use yamux::Stream;

use crate::errors::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthInfo {
    pub privilege_key: String,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginRequest {
    version: String,
    hostname: String,
    os: String,
    arch: String,
    user: String,
    metas: HashMap<String, String>,
    pool_count: i32,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    auth_info: Option<AuthInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PingRequest {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub(crate) auth_info: Option<AuthInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResp {
    version: String,
    pub run_id: Option<String>,
    pub error: Option<String>,
    pub login_message: Option<String>,
    #[serde(rename = "RxLimit")]
    rx_limit: Option<u64>,
    #[serde(rename = "TxLimit")]
    tx_limit: Option<u64>,
}

#[derive(Debug)]
pub struct LoginRespFrame {
    pub resp: LoginResp,
    //16 bytes iv
    pub iv: Vec<u8>,
}

impl LoginRequest {
    pub fn new(token: &str, user: &str) -> Self {
        let timestamp = Utc::now().timestamp();
        let privilege_key = get_privilege_key(timestamp, token);
        let metas = HashMap::new();
        Self {
            version: crate::FRP_VERSION.to_string(),
            hostname: "".to_string(),
            os: consts::OS.to_string(),
            arch: consts::ARCH.to_string(),
            user: user.to_string(),
            auth_info: Some(AuthInfo {
                privilege_key,
                timestamp,
            }),
            metas,
            pool_count: 1,
        }
    }

    pub async fn send_msg(&self, main_stream: &mut Stream) -> Result<LoginResp, Error> {
        let str = serde_json::to_string(&self)?;
        let frame = str.as_bytes();
        let hdr = MsgHeader::new(TYPE_LOGIN, frame.len() as u64);
        main_stream
            .write_all(&msg_header_encode(&hdr).to_vec())
            .await?;
        main_stream.write_all(&frame).await?;

        let mut msg_hdr = [0; MSG_HEADER_SIZE];
        main_stream.read_exact(&mut msg_hdr).await?;
        let header: MsgHeader = msg_header_decode(&msg_hdr.try_into().unwrap());
        let mut msg = vec![0; header.len as usize];
        main_stream.read_exact(&mut msg).await?;
        let resp = String::from_utf8_lossy(&msg);
        debug!("login_resp {:?}", resp);
        Ok(serde_json::from_str(&resp)?)
    }
}

pub fn get_privilege_key(timestamp: i64, auth_token: &str) -> String {
    let seed = format!("{}{}", auth_token, timestamp);
    let digest = md5::compute(seed);

    format!("{:x}", digest)
}

pub struct ReqWorkConn;


#[derive(Serialize, Deserialize, Debug)]
pub struct NewWorkConn {
    run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    privilege_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
}

impl NewWorkConn {
    pub fn from_run_id(run_id: &str) -> Self {
        Self {
            run_id: run_id.to_string(),
            privilege_key: None,
            timestamp: None,
        }
    }
    pub fn new(run_id: &str, token: &str) -> Self {
        let timestamp = Utc::now().timestamp();
        let privilege_key = get_privilege_key(timestamp, token);

        Self {
            run_id: run_id.to_string(),
            privilege_key: Some(privilege_key),
            timestamp: Some(timestamp),
        }
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StartWorkConnResp {
    pub proxy_name: String,
    src_addr: String,
    dst_addr: String,
    src_port: u16,
    dst_port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewProxyRequest {
    proxy_name: String,
    proxy_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subdomain: Option<String>,
}

impl NewProxyRequest {
    pub fn new(proxy_name: &str, proxy_type: &str) -> Self {
        Self {
            proxy_name: proxy_name.to_string(),
            proxy_type: proxy_type.to_string(),
            remote_port: None,
            custom_domains: None,
            subdomain: None,
        }
    }

    pub fn set_remote_port(&mut self, remote_port: u16) {
        self.remote_port = Some(remote_port)
    }

    pub fn set_custom_domains(&mut self, custom_domains: &Vec<String>) {
        self.custom_domains = Some(custom_domains.clone())
    }

    pub fn set_subdomain(&mut self, subdomain: &str) {
        self.subdomain = Some(subdomain.to_string())
    }

}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewProxyResp {
    proxy_name: String,
    remote_addr: String,
    error: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MsgHeader {
    pub msg_type: MsgType,
    pub len: u64,
}

impl MsgHeader {
    pub fn new(msg_type: MsgType, len: u64) -> Self {
        Self { msg_type, len }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MsgType(pub u8);

pub const TYPE_LOGIN: MsgType = MsgType('o' as u8);
pub const TYPE_LOGIN_RESP: MsgType = MsgType('1' as u8);
pub const TYPE_REQ_WORK_CONN: MsgType = MsgType('r' as u8);
pub const TYPE_PING: MsgType = MsgType('h' as u8);
pub const TYPE_NEW_WORK_CONN: MsgType = MsgType('w' as u8);

pub const TYPE_NEW_PROXY: MsgType = MsgType('p' as u8);
pub const TYPE_NEW_PROXY_RESP: MsgType = MsgType('2' as u8);

pub const TYPE_START_WORK_CONN: MsgType = MsgType('s' as u8);
pub const TypeCloseProxy: MsgType = MsgType('c' as u8);




pub const TypeNewVisitorConn: MsgType = MsgType('v' as u8);
pub const TypeNewVisitorConnResp: MsgType = MsgType('3' as u8);


pub const TypePong: MsgType = MsgType('4' as u8);

pub const TypeUDPPacket: MsgType = MsgType('u' as u8);

pub const TypeNatHoleVisitor: MsgType = MsgType('i' as u8);
pub const TypeNatHoleClient: MsgType = MsgType('n' as u8);
pub const TypeNatHoleResp: MsgType = MsgType('m' as u8);
pub const TypeNatHoleClientDetectOK: MsgType = MsgType('d' as u8);
pub const TypeNatHoleSid: MsgType = MsgType('5' as u8);

pub const MSG_HEADER_SIZE: usize = 9;

pub fn msg_header_encode(hdr: &MsgHeader) -> [u8; MSG_HEADER_SIZE] {
    let mut buf = [0; MSG_HEADER_SIZE];
    buf[0] = hdr.msg_type.0;
    buf[1..MSG_HEADER_SIZE].copy_from_slice(&hdr.len.to_be_bytes());

    buf
}

pub fn msg_header_decode(buf: &[u8; MSG_HEADER_SIZE]) -> MsgHeader {
    MsgHeader {
        msg_type: MsgType(buf[0]),
        len: u64::from_be_bytes([
            buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
        ]),
    }
}
