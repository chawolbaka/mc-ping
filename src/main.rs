#![allow(unused)]
mod protocol;
use crate::protocol::io::*;
use crate::protocol::packet::*;
use serde_json::Value;
use std::env;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::process::exit;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MINECRAFT_DEFAULT_PORT: &'static str = ":25565";

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let addr = resolve_host(&mut args[1]);
    for seq in (0..4) {
        match ping(&addr) {
            Ok(r) => println!("{}: seq={} online={} time={:?}", addr.ip(), seq, r.0, r.1),
            Err(e) => eprintln!("{}", e),
        }
    }
}

fn resolve_host(host: &mut String) -> SocketAddr {
    if !host.contains(':') {
        host.push_str(MINECRAFT_DEFAULT_PORT);
    }
    match host.to_socket_addrs() {
        Ok(mut addrs) => return addrs.next().unwrap(),
        Err(e) => {
            eprintln!("Ping request could not find host {}. Please check the name and try again.", strip_port(host));
            exit(-1);
        },
    };
    unreachable!()
}

fn strip_port(s: &str) -> &str {
    s.rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map(|(host, _)| host)
        .unwrap_or(s)
}

fn ping(addr: &SocketAddr) -> Result<(i64, Duration)> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write_packet(
        &mut HandshakePacket {
            protocol_version: -1,
            server_address: addr.to_string(),
            server_port: addr.port(),
            next_state: HandshakeState::Status,
        }
        .encode()?,
    );
    stream.write_packet(&mut PingRquestPacket {}.encode()?);

    let response = PingResponsePacket::decode(&stream.read_packet()?)?;

    // Ping <-> Pong
    let start = Instant::now();
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    stream.write_all(&mut PingPacket { code: time }.encode()?);
    stream.read_packet(); // The Pong code does not need to be verified.
    let elapsed = start.elapsed();

    let j: Value = serde_json::from_str(&response.content)?;
    let online = j["players"]["online"].as_i64().unwrap_or(-1);
    Ok((online, elapsed))
}
