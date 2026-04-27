use crate::protocol::io::{decode_forge_d, seek_varint_length, MinecraftReadExt, MinecraftWriteExt};
use crate::protocol::packet::{
    HandshakePacket, HandshakeState, Packet, PingPacket, PingRequestPacket, PingResponsePacket, PongPacket,
};
use serde_json::Value;
use std::io::{self, ErrorKind, Result};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PONG_PACKET_SIZE: usize = 10;

pub struct PingReport {
    pub received_bytes: usize,
    pub onlines: Option<i64>, // some servers may return a negative number
    pub mods: Option<usize>,
    pub elapsed: Duration,
    pub json: String,
}

pub fn ping(addr: &SocketAddr, host: &str, port: u16, timeout: Duration, verify: bool) -> Result<PingReport> {
    let mut stream = TcpStream::connect_timeout(addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    stream.write_packet(
        &HandshakePacket {
            protocol_version: -1,
            server_address: host.to_string(),
            server_port: port,
            next_state: HandshakeState::Status,
        }
        .encode()?,
    )?;
    let request = PingRequestPacket;
    stream.write_packet(&request.encode()?)?;

    let first_packet = stream.read_packet()?;
    let response = PingResponsePacket::decode(&first_packet)?;

    // Ping <-> Pong
    let start = Instant::now();
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    stream.write_packet(&PingPacket { code: time }.encode()?)?;
    let pong_packet = stream.read_packet()?;
    let elapsed = start.elapsed();
    if verify && PongPacket::decode(&pong_packet)?.code != time {
        return Err(io::Error::new(ErrorKind::InvalidData, "Invalid pong code"));
    }

    let len = first_packet.len();
    let json: Value = serde_json::from_str(&response.content)?;
    Ok(PingReport {
        received_bytes: seek_varint_length(len) + len + PONG_PACKET_SIZE,
        onlines: json["players"]["online"].as_i64(),
        mods: read_forge_mods(&json)?,
        elapsed,
        json: response.content,
    })
}

pub fn seek_send_bytes(host: &str) -> usize {
    let handshake_packet_len = 1 + 1 + seek_varint_length(host.len()) + host.len() + 2 + 1;
    seek_varint_length(handshake_packet_len) + handshake_packet_len + 2 + PONG_PACKET_SIZE
}

// ref: src/main/java/net/minecraftforge/network/ServerStatusPing.java (1.20.x)
fn read_forge_mods(json: &Value) -> Result<Option<usize>> {
    if let Some(d) = json.get("forgeData").and_then(|m| m.get("d")) {
        let units: Vec<u16> = d.as_str().unwrap_or_default().encode_utf16().collect();
        if units.len() < 3 {
            return Ok(None);
        }

        let bytes = decode_forge_d(&units)?;
        let mut cursor = &bytes[1..];
        let size = cursor.read_unsigned_short()? as usize;
        return Ok(Some(size));
    }

    Ok(json
        .get("modinfo")
        .and_then(|m| m.get("modList"))
        .and_then(|list| list.as_array())
        .map(|arr| arr.len()))
}
