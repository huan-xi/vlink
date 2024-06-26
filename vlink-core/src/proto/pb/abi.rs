#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RequireReply {
    #[prost(string, tag="1")]
    pub src: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub proto: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub server: ::prost::alloc::string::String,
}
///客户端->服务端
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToServer {
    #[prost(uint64, tag="1")]
    pub id: u64,
    #[prost(oneof="to_server::ToServerData", tags="2, 3, 4, 10, 11, 12, 13, 14, 16, 20")]
    pub to_server_data: ::core::option::Option<to_server::ToServerData>,
}
/// Nested message and enum types in `ToServer`.
pub mod to_server {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ToServerData {
        #[prost(message, tag="2")]
        Handshake(super::ReqHandshake),
        #[prost(message, tag="3")]
        ReqConfig(super::ReqConfig),
        #[prost(message, tag="4")]
        PeerForward(super::PeerForward),
        #[prost(message, tag="10")]
        PeerEnter(super::PeerEnter),
        #[prost(message, tag="11")]
        PeerLeave(super::PeerLeave),
        #[prost(message, tag="12")]
        PeerChange(super::PeerChange),
        #[prost(message, tag="13")]
        PeerMessage(super::PeerMessage),
        /// 上报信息
        #[prost(message, tag="14")]
        PeerReport(super::PeerReport),
        /// 更新端点
        #[prost(message, tag="16")]
        UpdateExtraEndpoint(super::ExtraEndpoint),
        #[prost(message, tag="20")]
        DevHandshakeComplete(super::DevHandshakeComplete),
    }
}
////节点转发
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerForward {
    #[prost(string, tag="1")]
    pub target_pub_key: ::prost::alloc::string::String,
    #[prost(oneof="peer_forward::Data", tags="2")]
    pub data: ::core::option::Option<peer_forward::Data>,
}
/// Nested message and enum types in `PeerForward`.
pub mod peer_forward {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag="2")]
        RequireReply(super::RequireReply),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DevHandshakeComplete {
    #[prost(string, tag="1")]
    pub target_pub_key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub proto: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExtraEndpoint {
    #[prost(string, tag="1")]
    pub proto: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub endpoint: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReqHandshake {
    #[prost(uint32, tag="1")]
    pub version: u32,
    ///访问token,用于身份校验
    #[prost(string, tag="2")]
    pub pub_key: ::prost::alloc::string::String,
    #[prost(string, optional, tag="3")]
    pub token: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag="4")]
    pub sign: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReqConfig {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerEnter {
    #[prost(string, tag="1")]
    pub ip: ::prost::alloc::string::String,
    //// udp 直连端口
    #[prost(string, optional, tag="2")]
    pub endpoint_addr: ::core::option::Option<::prost::alloc::string::String>,
    //// udp 端口
    #[prost(uint32, tag="3")]
    pub port: u32,
    #[prost(message, repeated, tag="4")]
    pub extra_endpoints: ::prost::alloc::vec::Vec<ExtraEndpoint>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerLeave {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerMessage {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerChange {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerReport {
}
///客户端->服务端
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToClient {
    /// 通信id
    #[prost(uint64, tag="1")]
    pub id: u64,
    #[prost(oneof="to_client::ToClientData", tags="2, 3, 4, 5, 6, 7, 8, 10")]
    pub to_client_data: ::core::option::Option<to_client::ToClientData>,
}
/// Nested message and enum types in `ToClient`.
pub mod to_client {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ToClientData {
        #[prost(message, tag="2")]
        Error(super::ToClientError),
        #[prost(message, tag="3")]
        RespServerInfo(super::RespServerInfo),
        #[prost(message, tag="4")]
        RespHandshake(super::RespHandshake),
        #[prost(message, tag="5")]
        RespConfig(super::RespConfig),
        #[prost(message, tag="6")]
        PeerEnter(super::BcPeerEnter),
        #[prost(message, tag="7")]
        PeerLeave(super::BcPeerLevel),
        #[prost(message, tag="8")]
        RequireReply(super::RequireReply),
        #[prost(message, tag="10")]
        UpdateExtraEndpoint(super::BcUpdateExtraEndpoint),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BcPeerLevel {
    #[prost(string, tag="1")]
    pub pub_key: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BcUpdateExtraEndpoint {
    #[prost(string, tag="1")]
    pub pub_key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub proto: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub endpoint: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToClientError {
    #[prost(int32, tag="1")]
    pub code: i32,
    #[prost(string, tag="2")]
    pub msg: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RespServerInfo {
    #[prost(string, tag="1")]
    pub version: ::prost::alloc::string::String,
    /// 服务端公钥
    #[prost(string, tag="2")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, optional, tag="3")]
    pub desc: ::core::option::Option<::prost::alloc::string::String>,
}
//// 握手响应
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RespHandshake {
    #[prost(bool, tag="1")]
    pub success: bool,
    #[prost(string, optional, tag="4")]
    pub msg: ::core::option::Option<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExtraTransport {
    #[prost(string, tag="1")]
    pub proto: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub params: ::prost::alloc::string::String,
}
//// 配置响应
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RespConfig {
    #[prost(int64, tag="1")]
    pub network_id: i64,
    /// 分配的网络地址
    #[prost(uint32, tag="2")]
    pub address: u32,
    /// 子网掩码
    #[prost(uint32, tag="3")]
    pub mask: u32,
    #[prost(uint32, tag="4")]
    pub network: u32,
    ///udp 端口
    #[prost(uint32, tag="5")]
    pub port: u32,
    #[prost(string, optional, tag="6")]
    pub ipv6_addr: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(message, repeated, tag="10")]
    pub peers: ::prost::alloc::vec::Vec<BcPeerEnter>,
    #[prost(message, repeated, tag="11")]
    pub extra_transports: ::prost::alloc::vec::Vec<ExtraTransport>,
    #[prost(message, repeated, tag="12")]
    pub peer_extra_transports: ::prost::alloc::vec::Vec<PeerExtraTransport>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerExtraTransport {
    #[prost(string, tag="1")]
    pub target_pub_key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub proto: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub endpoint: ::prost::alloc::string::String,
    #[prost(int32, tag="4")]
    pub index: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BcPeerEnter {
    #[prost(string, tag="1")]
    pub pub_key: ::prost::alloc::string::String,
    //// 网络中的ip
    #[prost(string, tag="2")]
    pub ip: ::prost::alloc::string::String,
    #[prost(string, optional, tag="4")]
    pub endpoint_addr: ::core::option::Option<::prost::alloc::string::String>,
    //// udp 端口
    #[prost(uint32, tag="5")]
    pub port: u32,
    //// 上次连接类型
    #[prost(enumeration="ConType", optional, tag="6")]
    pub last_con_type: ::core::option::Option<i32>,
    //// 连接模式
    #[prost(enumeration="ConnectionMode", tag="7")]
    pub mode: i32,
    #[prost(bool, tag="8")]
    pub is_online: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ConType {
    Udp = 0,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ConnectionMode {
    //// 主动连接
    Active = 0,
    //// 被动连接
    Passive = 1,
    //// 双向连接
    Bidirectional = 2,
    //// 禁止连接
    None = 3,
}
