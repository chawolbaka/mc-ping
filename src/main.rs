#![allow(unused)]
mod protocol;
use serde_json::Value;
use std::env;
use std::io::{ErrorKind, Read, Result, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::protocol::io::*;
use crate::protocol::packet::*;

fn main() {
    let args:Vec<String> = env::args().collect();
    let addr = SocketAddr::from_str(&args[1]).unwrap();
    for seq in (0..4) {
        let r = ping(&addr).unwrap();
        println!(
            "{}: seq={} online={} time={:?}",
            addr.ip(), seq, r.0, r.1
        );
    }
}

fn ping(addr: &SocketAddr) -> Result<(i64, Duration)> {
    let mut stream = TcpStream::connect(addr)?;

    send_packet(
        &mut stream,
        &mut HandshakePacket {
            protocol_version: -1,
            server_address: addr.to_string(),
            server_port: addr.port(),
            next_state: HandshakeState::Status,
        }
        .encode()?,
    );
    send_packet(&mut stream, &mut PingRquestPacket {}.encode()?);

    let response = PingResponsePacket::decode(&recv_packet(&mut stream)?)?;

    // Ping <-> Pong
    let start = Instant::now();
    send_packet(&mut stream, &mut PingPacket { code: SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() }.encode()?);
    recv_packet(&mut stream); // The Pong code does not need to be verified.
    let elapsed = start.elapsed();

    let j: Value = serde_json::from_str(&response.content)?;
    let online = j["players"]["online"].as_i64().unwrap_or(-1);
    Ok((online, elapsed))
}
