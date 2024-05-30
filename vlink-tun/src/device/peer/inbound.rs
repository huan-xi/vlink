use std::sync::Arc;
use log::{debug, error, info};
use crate::noise::handshake::IncomingInitiation;
use crate::noise::protocol;
use crate::noise::protocol::{COOKIE_REPLY_PACKET_SIZE, CookieReply, HANDSHAKE_RESPONSE_PACKET_SIZE, HandshakeResponse, TransportData};
use crate::Tun;
use crate::device::endpoint::Endpoint;
use crate::device::event::{DeviceEvent, HandshakeComplete};
use crate::device::inbound::OutboundSender;
use crate::device::peer::Peer;
use crate::device::peer::session::Session;

pub(super) async fn handle_handshake_initiation(
    peer: Arc<Peer>,
    endpoint: Box<dyn OutboundSender>,
    initiation: IncomingInitiation,
) {
    peer.monitor
        .traffic()
        .inbound(protocol::HANDSHAKE_INITIATION_PACKET_SIZE);
    let ret = {
        let mut handshake = peer.handshake.write().unwrap();
        handshake.respond(&initiation)
    };
    match ret {
        Ok((session, packet)) => {
            {
                let mut sessions = peer.sessions.write().unwrap();
                sessions.prepare_next(session);
            }
            peer.update_endpoint(endpoint.box_clone());
            endpoint.send(&packet).await.unwrap();
            peer.monitor.handshake().initiated();
        }
        Err(e) => debug!("failed to respond to handshake initiation: {e}"),
    }
}

pub(super) async fn handle_handshake_response(
    peer: Arc<Peer>,
    endpoint: Box<dyn OutboundSender>,
    packet: HandshakeResponse,
    _session: Session,
) {
    peer.monitor
        .traffic()
        .inbound(HANDSHAKE_RESPONSE_PACKET_SIZE);
    let ret = {
        let mut handshake = peer.handshake.write().unwrap();
        handshake.finalize(&packet)
    };
    match ret {
        Ok(session) => {
            let ret = {
                let mut sessions = peer.sessions.write().unwrap();
                sessions.complete_uninit(session)
            };
            if !ret {
                debug!("failed to complete handshake, session not found");
                return;
            }

            peer.monitor.handshake().completed();
            info!("handshake completed");
            peer.update_endpoint(endpoint);
            if let Some(e) = peer.endpoint.read().unwrap().as_ref() {
                let proto = e.protocol();
                peer.pub_event(DeviceEvent::HandshakeComplete(HandshakeComplete {
                    pub_key: peer.pub_key,
                    proto,
                }));
            }
            // let the peer know the session is valid
            peer.stage_outbound(vec![]).await;
        }
        Err(e) => debug!("failed to finalize handshake: {e}"),
    }
}

pub(super) async fn handle_cookie_reply(
    peer: Arc<Peer>,
    _endpoint: Box<dyn OutboundSender>,
    _packet: CookieReply,
    _session: Session,
) {
    peer.monitor.traffic().inbound(COOKIE_REPLY_PACKET_SIZE);
}

/// 传输数据
pub(super) async fn handle_transport_data(
    peer: Arc<Peer>,
    endpoint: Box<dyn OutboundSender>,
    packet: TransportData,
    session: Session,
) {
    peer.monitor.traffic().inbound(packet.packet_len());
    {
        let mut sessions = peer.sessions.write().unwrap();
        if sessions.complete_next(session.clone()) {
            info!("handshake completed");
            peer.monitor.handshake().completed();
        }
    }
    if !session.can_accept(packet.counter) {
        debug!("dropping packet due to replay");
        return;
    }

    peer.update_endpoint(endpoint);
    match session.decrypt_data(&packet) {
        Ok(data) => {
            if data.is_empty() {
                // keepalive
                return;
            }

            debug!("recv data from peer and try to send it to TUN");
            if let Err(e) = peer.tun.send(&data).await {
                error!("{peer} failed to send data to tun: {e}");
            }
            session.aceept(packet.counter);
        }
        Err(e) => debug!("failed to decrypt packet: {e}"),
    }
}