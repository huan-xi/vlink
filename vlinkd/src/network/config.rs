use std::fmt::{Debug, Display, Formatter};
use std::net::Ipv4Addr;
use base64::Engine;
use x25519_dalek::{PublicKey, StaticSecret};
use core::proto::pb::abi::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use vlink_tun::device::config::ArgConfig;
use vlink_tun::DeviceConfig;

pub struct PeerStaticSecret {
    pub private_key: StaticSecret,
    public_key: PublicKey,
}


impl Debug for PeerStaticSecret {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("pub:{}", self.base64_pub()).as_str())
    }
}


impl Serialize for PeerStaticSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(self.hex_private().as_str())
    }
}

impl<'de> Deserialize<'de> for PeerStaticSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let key = String::deserialize(deserializer)?;
        let bytes = hex::decode(key)
            .map_err(|e| D::Error::custom(e))?;
        let mut array: [u8; 32] = [0; 32];
        let slice = &bytes[..32];
        array.copy_from_slice(slice);
        let private_key = StaticSecret::from(array);
        let public_key = PublicKey::from(&private_key);
        Ok(PeerStaticSecret {
            private_key,
            public_key,
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StorageConfig {
    pub secret: PeerStaticSecret,
}


impl PeerStaticSecret {
    pub fn generate() -> Self {
        let private_key = StaticSecret::random_from_rng(rand_core::OsRng);
        let public_key = PublicKey::from(&private_key);
        Self {
            private_key,
            public_key,
        }
    }
    pub fn base64_pub(&self) -> String {
        crate::base64Encoding.encode(self.public_key)
    }
    pub fn hex_private(&self) -> String {
        hex::encode(self.private_key.as_ref())
    }
}

#[derive(Debug, Clone)]
pub struct PeersConfig {}

#[derive(Debug, Clone)]
pub struct VlinkNetworkConfig {
    pub tun_name: Option<String>,
    pub device_config: DeviceConfig,
    pub arg_config: ArgConfig,
    // pub test: RespConfig,
}

