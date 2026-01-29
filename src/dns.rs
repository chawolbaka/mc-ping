use std::io::{self, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs};

const MINECRAFT_DEFAULT_PORT: u16 = 25565;

pub fn resolve_host(host: &str) -> io::Result<SocketAddr> {
    let host = with_default_port(host);
    let mut addrs = host.to_socket_addrs()?;
    addrs.next().ok_or_else(|| {
        io::Error::new(
            ErrorKind::NotFound,
            format!(
                "Ping request could not find host {}. Please check the name and try again.",
                strip_port(&host)
            ),
        )
    })
}

pub fn strip_port(s: &str) -> &str {
    s.rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map(|(host, _)| host)
        .unwrap_or(s)
}

fn with_default_port(host: &str) -> String {
    if host.contains(':') {
        host.to_string()
    } else {
        format!("{host}:{MINECRAFT_DEFAULT_PORT}")
    }
}