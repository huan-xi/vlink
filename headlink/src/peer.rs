use crate::client::ClientConnect;

#[derive( Clone)]
pub struct VlinkPeer {
    pub(crate) connect: ClientConnect,
}