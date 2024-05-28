use std::ops::Deref;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use vlink_core::proto::pb::abi::BcPeerEnter;

#[derive(Debug)]
pub enum NetworkCtrlCmd {
    /// 请求查看网络信息
    ReqInfo,
    ChangeIp,
    PeerEnter(BcPeerEnter),
    FirstConnected,
    /// 重新加入网络
    Connected,
}

#[derive(Clone)]
pub struct NetworkCtrl {
    pub sender: mpsc::Sender<NetworkCtrlCmd>,
}


impl NetworkCtrl {
    pub fn new() -> (NetworkCtrl, Receiver<NetworkCtrlCmd>) {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        (Self { sender: tx }, rx)
    }

    /// 发送请求
    pub fn request(&self) {}
}

impl Deref for NetworkCtrl {
    type Target = mpsc::Sender<NetworkCtrlCmd>;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}
