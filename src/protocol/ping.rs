use crate::protocol::io::*;
use crate::protocol::packet::*;
use serde_json::Value;
use std::io::{Result};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct PingReport {
    pub received_bytes: usize,
    pub online: Option<i64>, // some server many return a negative number
    pub mods: Option<usize>,
    pub elapsed: Duration,
    pub json: String
}

pub fn ping(host: &str, addr: &SocketAddr) -> Result<PingReport> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write_packet(
        &mut HandshakePacket {
            protocol_version: -1,
            server_address: host.to_string(),
            server_port: addr.port(),
            next_state: HandshakeState::Status,
        }
        .encode()?,
    )?;
    stream.write_packet(&mut PingRquestPacket {}.encode()?)?;

    let first_packet = stream.read_packet()?;
    let response = PingResponsePacket::decode(&first_packet)?;

    // Ping <-> Pong
    let start = Instant::now();
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    stream.write_packet(&mut PingPacket { code: time }.encode()?)?;
    stream.read_packet()?; // The Pong code does not need to be verified.
    let elapsed = start.elapsed();

    let len = first_packet.len();
    let json: Value = serde_json::from_str(&response.content)?;
    Ok(PingReport {
        received_bytes: get_varint_length(len) + len + 10, /* Pong size */
        online: json["players"]["online"].as_i64(),
        mods: json
        .get("modinfo")
        .and_then(|m| m.get("modList"))
        .and_then(|list| list.as_array())
        .map(|arr| arr.len()),
        elapsed: elapsed,
        json: response.content
    })
}