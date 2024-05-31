use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use log::{debug, error, warn};
use crate::{LocalStaticSecret, Tun};
use crate::noise::handshake::{Cookie, IncomingInitiation};
use crate::noise::{Message, protocol};
use crate::device::DeviceInner;
use crate::device::peer::InboundEvent;
use crate::device::inbound::OutboundSender;

pub struct DeviceHandle {}

impl DeviceHandle {
    /// 新建并挂起
    pub fn spawn(token: CancellationToken, inner: Arc<DeviceInner>) -> Self {
        //入口数据

        //出口数据
        let inbound_loop = tokio::spawn(loop_inbound(token.child_token(), inner.clone()));
        let outbound_loop = tokio::spawn(loop_outbound(token.child_token(), inner.clone()));

        Self {}
    }
}

/// 循环处理outbound
async fn loop_outbound(token: CancellationToken, inner: Arc<DeviceInner>)

{
    debug!("Device outbound loop is UP");
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                debug!("Device outbound loop is DOWN");
                return;
            }
            _ = tick_outbound(Arc::clone(&inner)) => {}
        }
    }
}


async fn tick_outbound(inner: Arc<DeviceInner>)
{
    const IPV4_HEADER_LEN: usize = 20;
    const IPV6_HEADER_LEN: usize = 40;

    match inner.tun.recv().await {
        Ok(buf) => {
            let dst = {
                match buf[0] & 0xF0 {
                    0x40 if buf.len() < IPV4_HEADER_LEN => return,
                    0x40 => {
                        let addr: [u8; 4] = buf[16..20].try_into().unwrap();
                        IpAddr::from(Ipv4Addr::from(addr))
                    }
                    0x60 if buf.len() < IPV6_HEADER_LEN => return,
                    0x60 => {
                        let addr: [u8; 16] = buf[24..40].try_into().unwrap();
                        IpAddr::from(Ipv6Addr::from(addr))
                    }
                    n => {
                        debug!("unknown IP version: {}", n);
                        return;
                    }
                }
            };
            //macos 需要处理
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if dst == inner.tun_addr {
                //写回tun
                if let Err(e) = inner.tun.send(&buf).await {
                    error!("self data tun write error: {}", e);
                }
                return;
            }

            debug!("trying to send packet to {}", dst);
            let peer = inner.peers.read().unwrap().get_by_ip(dst);

            if let Some(peer) = peer {
                debug!("sending packet[{}] to {dst}", buf.len());
                peer.stage_outbound(buf).await
            } else {
                warn!("no peer found for {dst}");
            }
        }
        Err(e) => {
            error!("TUN read error: {}", e)
        }
    }
}


/// 处理设备endpoint入口数据
///
async fn loop_inbound(token: CancellationToken, inner: Arc<DeviceInner>)
{
    let mut transport = inner.settings.lock().unwrap().inbound.take_rx().expect("inbound transport is none");
    debug!("Device Inbound loop is UP");


    let (secret, cookie) = {
        // let settings = inner.settings.lock().unwrap();
        // (settings.secret.clone(), Arc::clone(&settings.cookie))
        inner.settings.lock().unwrap().secret_and_cookie()
    };

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                debug!("Device Inbound loop is DOWN");
                return;
            }
            //处理传输层数据
            data = transport.recv() => {
                if let Some((data, sender)) = data {
                    tick_inbound(Arc::clone(&inner), &secret, Arc::clone(&cookie), sender, data).await;
                }
            }
        }
    }
}


async fn tick_inbound(
    inner: Arc<DeviceInner>,
    secret: &LocalStaticSecret,
    cookie: Arc<Cookie>,
    endpoint: Box<dyn OutboundSender>,
    payload: Vec<u8>,
)
{
    if Message::is_handshake(&payload) {
        if !cookie.validate_mac1(&payload) {
            debug!("invalid mac1");
            return;
        }

        if !inner.rate_limiter.fetch_token() {
            debug!("rate limited");
            if !cookie.validate_mac2(&payload) {
                debug!("invalid mac2");
                return;
            }
            debug!("try to send cookie reply");
            let reply = cookie.generate_cookie_reply(&payload, endpoint.dst());
            endpoint.send(&reply).await.unwrap();
            return;
        }
    }

    match Message::parse(&payload) {
        Ok(Message::HandshakeInitiation(p)) => {
            let initiation = IncomingInitiation::parse(secret, &p).unwrap_or_else(|_| todo!());
            if let Some(peer) = inner.get_peer_by_key(initiation.static_public_key.as_bytes()) {
                peer.stage_inbound(InboundEvent::HanshakeInitiation {
                    endpoint,
                    initiation,
                })
                    .await;
            }
        }
        Ok(msg) => {
            let receiver_index = match &msg {
                Message::HandshakeResponse(p) => p.receiver_index,
                Message::CookieReply(p) => p.receiver_index,
                Message::TransportData(p) => p.receiver_index,
                _ => unreachable!(),
            };
            if let Some((session, peer)) = inner.get_session_by_index(receiver_index) {
                match msg {
                    Message::HandshakeResponse(packet) => {
                        peer.stage_inbound(InboundEvent::HandshakeResponse {
                            endpoint,
                            packet,
                            session,
                        })
                            .await;
                    }
                    Message::CookieReply(packet) => {
                        peer.stage_inbound(InboundEvent::CookieReply {
                            endpoint,
                            packet,
                            session,
                        })
                            .await;
                    }
                    Message::TransportData(packet) => {
                        if packet.counter > protocol::REJECT_AFTER_MESSAGES {
                            warn!("received too many messages from peer [index={receiver_index}]");
                            return;
                        }

                        peer.stage_inbound(InboundEvent::TransportData {
                            endpoint,
                            packet,
                            session,
                        })
                            .await;
                    }
                    _ => unreachable!(),
                }
            } else {
                warn!("received message from unknown peer [index={receiver_index}]");
            }
        }
        Err(e) => {
            warn!("failed to parse message type: {:?}", e);
        }
    }
}