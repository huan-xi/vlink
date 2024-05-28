use std::fmt::{Debug, Formatter};
use base64::Engine;
use x25519_dalek::{PublicKey, StaticSecret};
use base64::engine::general_purpose::STANDARD as base64Encoding;
use crypto_box::{Nonce, SalsaBox, SecretKey};
use crypto_box::aead::Aead;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use crate::proto::HELLO_STR;

#[derive(Clone)]
pub struct VlinkStaticSecret {
    pub private_key: StaticSecret,
    public_key: x25519_dalek::PublicKey,
}


impl VlinkStaticSecret {
    pub fn generate() -> Self {
        let private_key = StaticSecret::random_from_rng(rand_core::OsRng);
        let public_key = PublicKey::from(&private_key);
        Self {
            private_key,
            public_key,
        }
    }
    pub fn base64_pub(&self) -> String {
        base64Encoding.encode(self.public_key)
    }
    pub fn hex_private(&self) -> String {
        hex::encode(self.private_key.as_ref())
    }

    /// 私钥签名
    pub fn hello_sign(&self, target_pub: [u8; 32]) -> anyhow::Result<String> {
        let bob_public_key = crypto_box::PublicKey::from(target_pub);
        let alice_secret_key = SecretKey::from(*self.private_key.as_bytes());
        let nonce = &target_pub[..24];
        let no = Nonce::from_slice(nonce);
        let alice_box = SalsaBox::new(&bob_public_key, &alice_secret_key);
        let msg = HELLO_STR.as_bytes();
        let ciphertext = alice_box.encrypt(&no, &msg[..])
            .map_err(|e| anyhow::anyhow!("加密错误:{}",e))?;
        Ok(base64Encoding.encode(ciphertext))
    }
}

impl Debug for VlinkStaticSecret {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("pub:{}", self.base64_pub()).as_str())
    }
}


impl Serialize for VlinkStaticSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(self.hex_private().as_str())
    }
}

impl<'de> Deserialize<'de> for VlinkStaticSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let key = String::deserialize(deserializer)?;
        let bytes = hex::decode(key)
            .map_err(|e| D::Error::custom(e))?;
        let mut array: [u8; 32] = [0; 32];
        let slice = &bytes[..32];
        array.copy_from_slice(slice);
        let private_key = StaticSecret::from(array);
        let public_key = PublicKey::from(&private_key);
        Ok(VlinkStaticSecret {
            private_key,
            public_key,
        })
    }
}
