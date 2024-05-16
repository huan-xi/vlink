use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
use chacha20poly1305::aead::{Aead, Payload};
use crate::noise::Error;

#[derive(Clone)]
pub struct ChachaCipher {
    cipher: ChaCha20Poly1305,
}

impl ChachaCipher {
    //-> Result<Vec<u8>, Error>
    pub fn new(key: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            cipher: ChaCha20Poly1305::new_from_slice(key).map_err(|_| Error::InvalidKeyLength)?,
        })
    }
    #[inline]
    pub fn aead_encrypt(&self, counter: u64, msg: &[u8], aad: &[u8]) -> Result<Vec<u8>, Error> {
        let nonce = {
            let mut nonce = [0u8; 12];
            nonce[4..].copy_from_slice(&counter.to_le_bytes());
            nonce
        };
        self.cipher
            .encrypt(Nonce::from_slice(&nonce), Payload { msg, aad })
            .map_err(Error::Encryption)
    }
    #[inline]
    pub fn aead_decrypt(&self, counter: u64, msg: &[u8], aad: &[u8]) -> Result<Vec<u8>, Error> {
        let nonce = {
            let mut nonce = [0u8; 12];
            nonce[4..].copy_from_slice(&counter.to_le_bytes());
            nonce
        };
        self.cipher
            .decrypt(Nonce::from_slice(&nonce), Payload { msg, aad })
            .map_err(|_| Error::Decryption)
    }
}