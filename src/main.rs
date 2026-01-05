#![allow(unused)]
mod protocol;
use crate::protocol::io::*;
use crate::protocol::packet::*;
use clap::Parser;
use serde_json::Value;
use std::io::{Result, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::exit;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MINECRAFT_DEFAULT_PORT: &'static str = ":25565";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Hostname or IP address to ping
    target: String,

    /// Number of mc-ping requests to send
    #[arg(short = 'c', long, default_value_t = 4)]
    count: u32,
}
struct PingReport {
    received_bytes: usize,
    online: i64,
    mods: i64,
    elapsed: Duration,
}

fn main() {
    let args = Args::parse();
    let addr = resolve_host(&args.target);
    for seq in (0..args.count) {
        match ping(&args.target, &addr) {
            Ok(r) => {
                let mut info = String::from_str(&format!(
                    "{} bytes from {}: seq={} ", r.received_bytes, addr.ip(), seq)).unwrap();

                if r.online >= 0 {
                    info.push_str(&format!("onlines={} ",r.online));
                }

                if r.mods > 0 {
                    info.push_str(&format!("mods={} ",r.mods));
                }
                info.push_str(&format!("time={:?}",r.elapsed));
                println!("{info}");
            },
            Err(e) => eprintln!("{}", e),
        }
    }
}

fn ping(host: &str, addr: &SocketAddr) -> Result<PingReport> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write_packet(
        &mut HandshakePacket {
            protocol_version: -1,
            server_address: strip_port(host).to_string(),
            server_port: addr.port(),
            next_state: HandshakeState::Status,
        }
        .encode()?,
    );
    stream.write_packet(&mut PingRquestPacket {}.encode()?);

    let first_packet = stream.read_packet()?;
    let response = PingResponsePacket::decode(&first_packet)?;

    // Ping <-> Pong
    let start = Instant::now();
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    stream.write_all(&mut PingPacket { code: time }.encode()?);
    stream.read_packet(); // The Pong code does not need to be verified.
    let elapsed = start.elapsed();

    let len = first_packet.len();
    let json: Value = serde_json::from_str(&response.content)?;
    Ok(PingReport {
        received_bytes: get_varint_length(len) + len + 10, /* Pong size */
        online: json["players"]["online"].as_i64().unwrap_or(-1),
        mods: json
        .get("modinfo")
        .and_then(|m| m.get("modList"))
        .and_then(|list| list.as_array())
        .map(|arr| arr.len() as i64)
        .unwrap_or(-1),
        elapsed: elapsed,
    })
}

fn resolve_host(host: &str) -> SocketAddr {
    let mut host = String::from_str(host).unwrap();
    if !host.contains(':') {
        host.push_str(MINECRAFT_DEFAULT_PORT);
    }

    match host.to_socket_addrs() {
        Ok(mut addrs) => return addrs.next().unwrap(),
        Err(e) => {
            eprintln!(
                "Ping request could not find host {}. Please check the name and try again.",
                strip_port(&host)
            );
            exit(-1);
        }
    };
    unreachable!()
}

fn strip_port(s: &str) -> &str {
    s.rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map(|(host, _)| host)
        .unwrap_or(s)
}
