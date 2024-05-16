use base64::{DecodeError, Engine};
use base64::engine::general_purpose::STANDARD as base64Encoding;

pub fn decode_base64(s: &str) -> Result<Vec<u8>, DecodeError> {
    base64Encoding.decode(s)
}