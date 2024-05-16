///客户端->服务端
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToServer {
    #[prost(uint64, tag="1")]
    pub id: u64,
    #[prost(oneof="to_server::ToServerData", tags="2, 3, 10, 11, 12, 13, 14")]
    pub to_server_data: ::core::option::Option<to_server::ToServerData>,
}
/// Nested message and enum types in `ToServer`.
pub mod to_server {
    #[derive(PartialOrd)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ToServerData {
        #[prost(message, tag="2")]
        Handshake(super::ReqHandshake),
        #[prost(message, tag="3")]
        ReqConfig(super::ReqConfig),
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
    }
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReqHandshake {
    #[prost(uint32, tag="1")]
    pub version: u32,
    ///访问token,用于身份校验
    #[prost(string, tag="2")]
    pub pub_key: ::prost::alloc::string::String,
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReqConfig {
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerEnter {
    #[prost(string, tag="1")]
    pub ip: ::prost::alloc::string::String,
    //// 短的地址
    #[prost(string, tag="2")]
    pub endpoint: ::prost::alloc::string::String,
    //// udp 端口
    #[prost(uint32, tag="3")]
    pub port: u32,
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerLeave {
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerMessage {
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerChange {
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerReport {
}
///客户端->服务端
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToClient {
    /// 通信id
    #[prost(uint64, tag="1")]
    pub id: u64,
    #[prost(oneof="to_client::ToClientData", tags="2, 3, 4")]
    pub to_client_data: ::core::option::Option<to_client::ToClientData>,
}
/// Nested message and enum types in `ToClient`.
pub mod to_client {
    #[derive(PartialOrd)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ToClientData {
        #[prost(message, tag="2")]
        RespHandshake(super::RespHandshake),
        #[prost(message, tag="3")]
        RespConfig(super::RespConfig),
        #[prost(message, tag="4")]
        PeerEnter(super::BcPeerEnter),
    }
}
//// 握手响应
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RespHandshake {
    #[prost(bool, tag="1")]
    pub success: bool,
    #[prost(string, optional, tag="4")]
    pub msg: ::core::option::Option<::prost::alloc::string::String>,
}
//// 配置响应
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RespConfig {
    #[prost(string, tag="1")]
    pub network_id: ::prost::alloc::string::String,
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
}
#[derive(PartialOrd)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BcPeerEnter {
    #[prost(string, tag="1")]
    pub pub_key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub ip: ::prost::alloc::string::String,
    //// udp 端口
    #[prost(uint32, tag="3")]
    pub port: u32,
}
