mod cookie;
mod initiation;
mod response;

pub const CONSTRUCTION: [u8; 37] = *b"Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s";
pub const IDENTIFIER: [u8; 34] = *b"WireGuard v1 zx2c4 Jason@zx2c4.com";
pub const LABEL_MAC1: [u8; 8] = *b"mac1----";
pub const LABEL_COOKIE: [u8; 8] = *b"cookie--";

pub use cookie::{Cookie, MacGenerator};
pub use initiation::{IncomingInitiation, OutgoingInitiation};
pub use response::{IncomingResponse, OutgoingResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::crypto::{LocalStaticSecret, PeerStaticSecret};
    use crate::noise::protocol::{HandshakeInitiation, HandshakeResponse};

    #[inline]
    fn gen_2_static_key() -> (PeerStaticSecret, PeerStaticSecret) {
        let p1_local = LocalStaticSecret::random();
        let p2_local = LocalStaticSecret::random();
        let mut p1_secret = p1_local.clone().with_peer(p2_local.public_key().to_bytes());
        let mut p2_secret = p2_local.with_peer(p1_local.public_key().to_bytes());
        let psk = PeerStaticSecret::random_psk();
        p1_secret.set_psk(psk);
        p2_secret.set_psk(psk);

        (p1_secret, p2_secret)
    }

    #[test]
    fn handshake_initiation() {
        let (p1_key, p2_key) = gen_2_static_key();
        let (p1_i, _p2_i) = (42, 88);
        let mut p1_cookie = MacGenerator::new(&p2_key);

        let (init_out, payload) = OutgoingInitiation::new(p1_i, &p1_key, &mut p1_cookie);
        let packet = HandshakeInitiation::try_from(payload.as_slice()).unwrap();
        let init_in = IncomingInitiation::parse(p2_key.local(), &packet).unwrap();

        assert_eq!(init_in.index, p1_i);
        assert_eq!(init_out.hash, init_in.hash);
        assert_eq!(init_out.chaining_key, init_in.chaining_key);
    }

    #[test]
    fn handshake_response() {
        let (p1_key, p2_key) = gen_2_static_key();
        let (p1_i, p2_i) = (42, 88);
        let mut p1_cookie = MacGenerator::new(&p2_key);
        let mut p2_cookie = MacGenerator::new(&p1_key);

        let (init_out, payload) = OutgoingInitiation::new(p1_i, &p1_key, &mut p1_cookie);
        let packet = HandshakeInitiation::try_from(payload.as_slice()).unwrap();
        let init_in = IncomingInitiation::parse(p2_key.local(), &packet).unwrap();

        assert_eq!(init_out.hash, init_in.hash);
        assert_eq!(init_out.chaining_key, init_in.chaining_key);

        let (resp_out, payload) = OutgoingResponse::new(&init_in, p2_i, &p2_key, &mut p2_cookie);
        let packet = HandshakeResponse::try_from(payload.as_slice()).unwrap();
        let resp_in = IncomingResponse::parse(&init_out, &p1_key, &packet).unwrap();

        assert_eq!(resp_in.index, p2_i);
        assert_eq!(resp_out.chaining_key, resp_in.chaining_key);
        assert_eq!(resp_out.hash, resp_in.hash);
    }
}
