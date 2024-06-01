use std::sync::Arc;
use tokio::sync::broadcast;
use crate::device::peer::Peer;
use crate::noise::crypto::PublicKey;

#[derive(Clone, Debug)]
pub struct ExtraEndpointSuccess {
    pub proto: String,
    pub endpoint: String,
}

#[derive(Clone, Debug)]
pub struct HandshakeComplete {
    pub pub_key: PublicKey,
    pub proto: String,
}


#[derive(Clone, Debug)]
pub enum DeviceEvent {
    HandshakeComplete(HandshakeComplete),

    ExtraEndpointSuccess(ExtraEndpointSuccess),
    NoEndpoint((PublicKey, String)),
    /// 节点未握手
    SessionFailed(Arc<Peer>),

    /// 传输层协议失败
    TransportFailed(Arc<Peer>),
    // HandshakeTimeout,
}

pub type DevicePublisher = broadcast::Sender<DeviceEvent>;
pub type DeviceSubscriber = broadcast::Receiver<DeviceEvent>;