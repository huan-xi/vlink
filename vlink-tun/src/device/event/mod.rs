use tokio::sync::broadcast;
use crate::noise::crypto::PublicKey;

#[derive(Clone, Debug)]
pub struct ExtraEndpointSuccess{
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
    // HandshakeTimeout,

}

pub type DevicePublisher = broadcast::Sender<DeviceEvent>;
pub type DeviceSubscriber = broadcast::Receiver<DeviceEvent>;