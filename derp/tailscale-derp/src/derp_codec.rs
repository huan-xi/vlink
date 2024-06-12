use bytes::{Buf, BufMut, BytesMut};
use httparse::Response;
use log::{debug, error, info};
use tokio_util::codec::{Decoder, Encoder};
use crate::errors::Error;
use std::fmt::Write;
use crypto_box::{CryptoBox, PublicKey, SalsaBox, SecretKey};
use crypto_box::aead::{Aead, AeadCore, Nonce};
use num_enum::{FromPrimitive, IntoPrimitive};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

const MAGIC: &str = "DERP🔑";
// 8 bytes: 0x44 45 52 50 f0 9f 94 91
const PROTOCOL_VERSION: u8 = 2;
const KEY_LEN: usize = 32;
// const (
// 	nonceLen       = 24
// 	frameHeaderLen = 1 + 4 // frameType byte + 4 byte length
// 	KEY_LEN         = 32
// 	maxInfoLen     = 1 << 20
// 	keepAlive      = 60 * time.Second
// )

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    version: u8,
}

/// 兼容ws 响应
#[derive(Debug)]
pub enum DerpResponse {
    ///ws 响应
    Ws,
    FrameServerKey([u8; 32]),
    FrameRecvPacket(([u8; 32], Vec<u8>)),
    FrameKeepAlive,
    ServerInfo(ServerInfo),
    Test,
}

#[derive(Debug, Clone)]
pub struct Header {
    cmd_type: CmdType,
    len: u32,
}

pub enum DerpRequest {
    ClientInfo(ClientInfo),
    SendPacket((Vec<u8>, Vec<u8>)),
    Test,
}

#[derive(Serialize, Deserialize)]
pub struct ClientInfo {
    version: u8,
    // #[serde(rename = "meshKey")]
    #[serde(skip)]
    mesh_key: Option<String>,
    #[serde(rename = "CanAckPings")]
    can_ack_pings: bool,
    #[serde(skip)]
    is_prober: bool,
}

impl ClientInfo {
    pub fn new(can_ack_pings: bool) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            mesh_key: None,
            can_ack_pings,
            is_prober: false,
        }
    }
}


pub struct DerpCodec {
    //发送到ws
    to_ws_response: bool,
    private_key: [u8; 32],
    public_key: [u8; 32],
    salsa_box: Option<SalsaBox>,
    header: Option<Header>,
}

impl DerpCodec {
    pub fn new(public_key: [u8; 32], private_key: [u8; 32], fast_connect: bool) -> Self {
        Self {
            to_ws_response: !fast_connect,
            private_key,
            public_key,
            salsa_box: None,
            header: None,
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum CmdType {
    FrameServerKey = 0x01,
    FrameClientInfo = 0x02,
    FrameServerInfo = 0x03,
    // 32B dest pub key + packet bytes
    FrameSendPacket = 0x04,
    //frameType(0x05) // v0/1: packet bytes, v2: 32B src pub key + packet bytes
    FrameRecvPacket = 0x05,
    // frameRecvPacket    =

    // no payload, no-op (to be replaced with ping/pong)
    FrameKeepAlive = 0x06,
// frameNotePreferred = frameType(0x07) // 1 byte payload: 0x01 or 0x00 for whether this is client's home node

// framePeerGone is sent from server to client to signal that
// a previous sender is no longer connected. That is, if A
// sent to B, and then if A disconnects, the server sends
// framePeerGone to B so B can forget that a reverse path
// exists on that connection to get back to A. It is also sent
// if A tries to send a CallMeMaybe to B and the server has no
// record of B (which currently would only happen if there was
// a bug).
// framePeerGone = frameType(0x08) // 32B pub key of peer that's gone + 1 byte reason

    // framePeerPresent is like framePeerGone, but for other
// members of the DERP region when they're meshed up together.
// framePeerPresent = frameType(0x09) // 32B pub key of peer that's connected + optional 18B ip:port (16 byte IP + 2 byte BE uint16 port)

    // 32B src pub key + 32B dst pub key + packet bytes
    FrameForwardPacket = 0x0a,


// frameWatchConns is how one DERP node in a regional mesh
// subscribes to the others in the region.
// There's no payload. If the sender doesn't have permission, the connection
// is closed. Otherwise, the client is initially flooded with
// framePeerPresent for all connected nodes, and then a stream of
// framePeerPresent & framePeerGone has peers connect and disconnect.
// frameWatchConns = frameType(0x10)

// frameClosePeer is a privileged frame type (requires the
// mesh key for now) that closes the provided peer's
// connection. (To be used for cluster load balancing
// purposes, when clients end up on a non-ideal node)
// frameClosePeer = frameType(0x11) // 32B pub key of peer to close.

// framePing = frameType(0x12) // 8 byte ping payload, to be echoed back in framePong
// framePong = frameType(0x13) // 8 byte payload, the contents of the ping being replied to

// frameHealth is sent from server to client to tell the client
// if their connection is unhealthy somehow. Currently the only unhealthy state
// is whether the connection is detected as a duplicate.
// The entire frame body is the text of the error message. An empty message
// clears the error state.
// frameHealth = frameType(0x14)

    // frameRestarting is sent from server to client for the
// server to declare that it's restarting. Payload is two big
// endian uint32 durations in milliseconds: when to reconnect,
// and how long to try total. See ServerRestartingMessage docs for
// more details on how the client should interpret them.
// frameRestarting = frameType(0x15)
    #[num_enum(default)]
    Unknown,
}

impl DerpCodec {
    /// 解码derp 数据
    fn decode_frame(&mut self, header: Header, data: BytesMut) -> Result<Option<DerpResponse>, Error> {
        //readFrameHeader
        // info!("decode header:{:?}", header);
        match header.cmd_type {
            CmdType::FrameServerKey => {
                info!("FrameServerKey");
                let mut server_key_bytes = [0; 32];
                server_key_bytes.copy_from_slice(&data[MAGIC.as_bytes().len()..]);
                let server_key = PublicKey::from_slice(&server_key_bytes)
                    .map_err(|_| Error::InvalidServerKey)?;
                let skey = SecretKey::from_slice(&self.private_key)
                    .map_err(|_| Error::InvalidKey)?;
                self.salsa_box = Some(SalsaBox::new(&server_key, &skey));
                return Ok(Some(DerpResponse::FrameServerKey(server_key_bytes)));
            }
            CmdType::FrameServerInfo => {
                //{"version":2}
                let data = self.decrypt_data(data.iter().as_slice())?;
                info!("FrameServerInfo :{}",String::from_utf8_lossy(data.as_slice()));
                let server_info: ServerInfo = serde_json::from_slice(data.as_slice())
                    .map_err(|_| Error::ErrorMsg("server info error".to_string()))?;
                return Ok(Some(DerpResponse::ServerInfo(server_info)));
            }
            CmdType::FrameKeepAlive => {
                return Ok(Some(DerpResponse::FrameKeepAlive));
            }
            CmdType::FrameRecvPacket => {
                let src_key = data[..KEY_LEN].try_into().unwrap();
                let data = data[KEY_LEN..].to_vec();
                return Ok(Some(DerpResponse::FrameRecvPacket((src_key, data))));
                // let data = self.decrypt_data(data.iter().as_slice())?;
                // info!("FrameRecvPacket :{}",String::from_utf8_lossy(data.iter().as_slice()));
            }
            _ => {
                info!("unknown frame type: {:?}", header.cmd_type);
            }
        };
        Ok(Some(DerpResponse::Test))
    }

    /// 加密数据
    fn encrypt_data(&self, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let nonce = SalsaBox::generate_nonce(&mut OsRng);
        Ok((nonce.to_vec(), self.salsa_box.as_ref()
            .ok_or(Error::NoServerKey)?
            .encrypt(&nonce, data)
            .map_err(|_| Error::EncryptError)?))
    }
    /// 解密数据

    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut nonce = Nonce::<SalsaBox>::default();
        nonce.copy_from_slice(&data[..24]);

        self.salsa_box.as_ref()
            .ok_or(Error::NoServerKey)?
            .decrypt(&nonce, &data[24..])
            .map_err(|_| Error::DecryptError)
    }
}


impl Decoder for DerpCodec {
    type Item = DerpResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // info!("decode:{:?}", src.iter().as_slice());
        if self.to_ws_response {
            //ws 响应阶段
            if let Some(response_len) = validate_server_response(src)? {
                //出去这部分响应数据
                src.advance(response_len);
                self.to_ws_response = false;
                Ok(Some(DerpResponse::Ws))
            } else {
                //数据不全继续等待数据
                Ok(None)
            }
        } else {
            //接受derp 数据
            let header = match self.header.take() {
                None => {
                    if src.len() < 5 {
                        return Ok(None);
                    };
                    let header = src.split_to(5);
                    let tb = CmdType::from(header[0]);
                    if tb == CmdType::Unknown {
                        error!("unknown frame type: {:?}", header[0]);
                    }
                    let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
                    if len > 1 << 20 {
                        return Err(Error::MaxFrameLength(len));
                    }
                    Header { cmd_type: tb, len: len as u32 }
                }
                Some(s) => { s }
            };
            if src.len() < header.len as usize {
                self.header = Some(header);
                return Ok(None);
            };
            let data = src.split_to(header.len as usize);
            return self.decode_frame(header, data);
        }
    }
}


fn validate_server_response(data: &[u8]) -> crate::Result<Option<usize>> {
    // let a = String::from_utf8_lossy(&data[172..]);
    // info!("validate_server_response:{:?}", a);
    let mut headers = [httparse::EMPTY_HEADER; 20];
    let mut response = Response::new(&mut headers);
    let status = response.parse(data)?;
    if !status.is_complete() {
        return Ok(None);
    }

    let response_len = status.unwrap();
    let code = response.code.unwrap();
    if code != 101 {
        let mut error_message = format!("server responded with HTTP error {code}", code = code);

        if let Some(reason) = response.reason {
            write!(error_message, ": {:?}", reason).expect("formatting reason failed");
        }

        return Err(error_message.into());
    }
    debug!("server response: {:?}", response);


    Ok(Some(response_len))
}


impl Encoder<DerpRequest> for DerpCodec {
    type Error = Error;

    fn encode(&mut self, item: DerpRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            DerpRequest::ClientInfo(info) => {
                //发送信息
                let info_json = serde_json::to_string(&info)
                    .map_err(|_| Error::EncodeError)?;
                let (nonce, bytes) = self.encrypt_data(info_json.as_bytes())?;
                //nonce+ data + public_key
                let mut buf = BytesMut::with_capacity(KEY_LEN + bytes.len() + nonce.len());
                buf.extend_from_slice(self.public_key.as_slice());
                buf.extend_from_slice(nonce.as_slice());
                buf.extend_from_slice(bytes.as_slice());
                write_frame(dst, CmdType::FrameClientInfo, buf.as_ref());
            }

            // 发送数据包到目标地址
            DerpRequest::SendPacket((dst_key, data)) => {
                //发送数据
                let mut buf = BytesMut::with_capacity(KEY_LEN + data.len());
                buf.extend_from_slice(dst_key.as_slice());
                buf.extend_from_slice(data.as_slice());
                write_frame(dst, CmdType::FrameSendPacket, buf.as_ref());
            }
            DerpRequest::Test => {}
        }
        Ok(())
    }
}

/// cmd+len+data
fn write_frame(dst: &mut BytesMut, cmd_type: CmdType, data: &[u8]) {
    dst.put_u8(cmd_type.into());
    dst.put_u32(data.len() as u32);
    dst.put(data);
}