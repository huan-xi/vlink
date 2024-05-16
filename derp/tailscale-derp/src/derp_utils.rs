use std::net::SocketAddr;
use httparse::Response;
use log::debug;
use url::Url;
use crate::errors::Error;

macro_rules! writeok {
    ($dst:expr, $($arg:tt)*) => {
        let _ = std::fmt::Write::write_fmt(&mut $dst, format_args!($($arg)*));
    }
}

pub(crate) fn build_request(url: &Url, headers: &[(String, String)]) -> String {
    let mut s = String::new();
    writeok!(s, "GET {path}", path = url.path());
    if let Some(query) = url.query() {
        writeok!(s, "?{query}", query = query);
    }

    s += " HTTP/1.1\r\n";

    if let Some(host) = url.host() {
        writeok!(s, "Host: {host}", host = host);
        if let Some(port) = url.port_or_known_default() {
            writeok!(s, ":{port}", port = port);
        }

        s += "\r\n";
    }

    writeok!(
        s,
        "Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\n"
    );

    for (name, value) in headers {
        writeok!(s, "{name}: {value}\r\n", name = name, value = value);
    }

    writeok!(s, "\r\n");
    s
}


pub(crate) fn resolve(url: &Url) -> Result<SocketAddr, Error> {
    url.socket_addrs(|| None)?
        .into_iter()
        .next()
        .ok_or_else(|| "can't resolve host".to_owned().into())
}