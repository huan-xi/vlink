use std::num::NonZeroU32;
use aes::Aes128;
use aes::cipher::KeyInit;
use cfb_mode::{BufDecryptor, BufEncryptor};
use cfb_mode::cipher::KeyIvInit;
use log::info;
use ring::pbkdf2;
use crate::errors::Error;

type Aes128CfbEnc = BufEncryptor<Aes128>;
type Aes128CfbDec = BufDecryptor<Aes128>;

static PBKDF2_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA1;
const DEFAULT_SALT: &str = "frp";

pub struct FrpBufCrypto {
    pub iv: Vec<u8>,
    key:    [u8; 16],
    cipher: Aes128,
    enc: Aes128CfbEnc,
    dec: Aes128CfbDec,
}

impl FrpBufCrypto {
    pub fn new(token: &str, iv: &[u8]) -> Result<Self, Error> {
        let mut key = [0x00; 16];
        pbkdf2::derive(PBKDF2_ALG, NonZeroU32::new(64).unwrap(), DEFAULT_SALT.as_bytes(), token.as_bytes(), &mut key);
        let cipher = Aes128::new_from_slice(key.as_slice())
            .map_err(|e| Error::Encrypt(e.to_string()))?;
        Ok(Self {
            key,
            cipher,
            iv: iv.to_vec(),
            enc: Aes128CfbEnc::new_from_slices(&key, &iv).map_err(|e| Error::Encrypt(e.to_string()))?,
            dec: Aes128CfbDec::new_from_slices(&key, &iv).map_err(|e| Error::Encrypt(e.to_string()))?,
        })
    }

    pub fn decrypt_buf(&mut self, buf: &mut [u8]) {
        self.dec.decrypt(buf)
    }
    pub fn encrypt_buf(&mut self, buf: &mut [u8]) {
        self.enc.encrypt(buf);
    }
}