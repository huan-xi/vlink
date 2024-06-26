pub mod pb;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// 握手时hello 字符串签名
pub const HELLO_STR: &str = "hello";

///  u32 + protobuf
pub fn bind_transport<T: AsyncRead + AsyncWrite>(stream: T) -> Framed<T, LengthDelimitedCodec> {
    let codec = LengthDelimitedCodec::builder()
        .length_field_offset(0)
        .length_field_type::<u32>()
        .new_codec();
    Framed::new(stream, codec)
}
