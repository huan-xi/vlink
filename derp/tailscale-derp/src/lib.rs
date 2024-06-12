pub mod derp_client;
pub mod errors;
mod derp_codec;
pub use derp_codec::DerpResponse;
pub use derp_codec::DerpRequest;

mod derp_utils;
mod ssl;
mod test_nacl;

use futures_util::StreamExt;
use log::{error, info};
use tokio::io::AsyncReadExt;
use crate::errors::Error;

// MAX_PACKET_SIZE is the maximum size of a packet sent over DERP.
// (This only includes the data bytes visible to magicsock, not
// including its on-wire framing overhead)
const MAX_PACKET_SIZE: usize = 64 << 10;

// FAST_START_HEADER is the header (with value "1") that signals to the HTTP
// server that the DERP HTTP client does not want the HTTP 101 response
// headers and it will begin writing & reading the DERP protocol immediately
// following its HTTP request.
const FAST_START_HEADER: &str = "Derp-Fast-Start";

type Result<T> = std::result::Result<T, Error>;

