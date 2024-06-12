use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_util::sync::CancellationToken;
use log::{debug, info, warn};
use crate::device::event::DeviceEvent;
use crate::Tun;
use crate::device::peer::{inbound, InboundEvent, InboundRx, OutboundEvent, OutboundRx, Peer};

pub struct PeerHandle {
    token: CancellationToken,
    handles: Vec<JoinHandle<()>>,
}

impl PeerHandle {
    /// peer 任务, 定时握手, 处理入口数据, 发送数据
    pub fn spawn(token: CancellationToken,
                 peer: Arc<Peer>,
                 inbound: InboundRx,
                 outbound: OutboundRx, ) -> Self {
        let handshake_loop = tokio::spawn(loop_handshake(token.child_token(), Arc::clone(&peer)));

        let inbound_loop = tokio::spawn(loop_inbound(
            token.child_token(),
            Arc::clone(&peer),
            inbound,
        ));
        let outbound_loop = tokio::spawn(loop_outbound(
            token.child_token(),
            Arc::clone(&peer),
            outbound,
        ));


        Self {
            token,
            // handles: vec![handshake_loop, inbound_loop, outbound_loop],
            handles: vec![],
        }
    }

    /// Cancel the background tasks and wait until they are terminated.
    /// If the timeout is reached, the tasks are terminated immediately.
    pub async fn cancel(mut self, timeout: Duration) {
        self.token.cancel();
        let handles = self.handles.drain(..).collect::<Vec<_>>();
        let abort_handles = handles.iter().map(|h| h.abort_handle()).collect::<Vec<_>>();
        if let Err(e) = tokio::time::timeout(timeout, join_all(handles)).await {
            warn!(
                "failed to cancel peer tasks in {}ms: {}",
                timeout.as_millis(),
                e
            );
            for handle in abort_handles {
                handle.abort();
            }
        }
    }
}


// Send to tun if we have a valid session
async fn loop_inbound(token: CancellationToken, peer: Arc<Peer>, mut rx: InboundRx)

{
    debug!("Inbound loop for {peer} is UP");

    loop {
        tokio::select! {
            () = token.cancelled() => break,
            event = rx.recv() => {
                match event {
                    Some(event) => tick_inbound(Arc::clone(&peer), event).await,
                    None => break,
                }
            }
        }
    }

    debug!("Inbound loop for {peer} is DOWN");
}

/// 处理peer 的入口数据

async fn tick_inbound(peer: Arc<Peer>, event: InboundEvent)
{
    match event {
        InboundEvent::HanshakeInitiation {
            endpoint,
            initiation,
        } => inbound::handle_handshake_initiation(Arc::clone(&peer), endpoint, initiation).await,
        InboundEvent::HandshakeResponse {
            endpoint,
            packet,
            session,
        } => inbound::handle_handshake_response(Arc::clone(&peer), endpoint, packet, session).await,
        InboundEvent::CookieReply {
            endpoint,
            packet,
            session,
        } => inbound::handle_cookie_reply(Arc::clone(&peer), endpoint, packet, session).await,
        InboundEvent::TransportData {
            endpoint,
            packet,
            session,
        } => inbound::handle_transport_data(Arc::clone(&peer), endpoint, packet, session).await,
    }
}

/// 循环与peer 握手
async fn loop_handshake(token: CancellationToken, peer: Arc<Peer>)
{
    debug!("Handshake loop for {peer} is UP");
    while !token.is_cancelled() {
        // tokio::time::sleep(Duration::from_secs(1)).await;
        peer.await_online().await;
        // todo 等待endpoint

        //等待设备变成在线
        if peer.monitor.can_handshake() {
            info!("initiating handshake");
            let packet = {
                let (next, packet) = peer.handshake.write().unwrap().initiate();
                let mut sessions = peer.sessions.write().unwrap();
                sessions.prepare_uninit(next);
                packet
            };
            // 直接发送握手包
            peer.send_outbound(&packet).await;
            peer.monitor.handshake().initiated();
        }

        time::sleep_until(peer.monitor.handshake().will_initiate_in().into()).await;
    }
    debug!("Handshake loop for {peer} is DOWN");
}


// Send to endpoint if connected, otherwise queue for later
async fn loop_outbound(token: CancellationToken, peer: Arc<Peer>, mut rx: OutboundRx)

{
    debug!("Outbound loop for {peer} is UP");

    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            _ = time::sleep_until(peer.monitor.keepalive().next_attempt_in(peer.monitor.traffic()).into()) => {
                peer.keepalive().await;
            }
            event = rx.recv() => {
                match event {
                    Some(OutboundEvent::Data(data)) => {
                        tick_outbound(Arc::clone(&peer), data).await;
                    }
                    None => break,
                }
            }
        }
    }

    debug!("Outbound loop for {peer} is DOWN");
}

#[inline]
async fn tick_outbound(peer: Arc<Peer>, data: Vec<u8>)
{
    let session = { peer.sessions.read().unwrap().current().clone() };
    let session = if let Some(s) = session { s } else {
        peer.pub_event(DeviceEvent::SessionFailed(peer.clone()));
        return;
    };

    match session.encrypt_data(&data) {
        Ok(packet) => {
            let buf = packet.to_bytes();
            peer.send_outbound(&buf).await;
            peer.monitor.traffic().outbound(buf.len());
        }
        Err(e) => {
            warn!("failed to encrypt packet: {}", e);
        }
    }
}