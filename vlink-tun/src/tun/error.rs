#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("interface must be named utun[0-9]*")]
    InvalidName,
    #[error("system call failed: {0}")]
    IO(#[from] std::io::Error),

    #[cfg(unix)]
    #[error("system call errno: {0}")]
    Sys(#[from] nix::Error),
    #[error("invalid IP packet")]
    InvalidIpPacket,
    #[error("tun closed")]
    Closed,
    #[error("unsupported {0}")]
    Unsupported(String)
}
