use crate::protocol::io::*;
use crate::protocol::packet::*;
use serde_json::Value;
use std::io::Result;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct PingReport {
    pub received_bytes: usize,
    pub onlines: Option<i64>, // some server many return a negative number
    pub mods: Option<usize>,
    pub elapsed: Duration,
    pub json: String,
}

pub fn ping(host: &str, addr: &SocketAddr, timeout: Duration) -> Result<PingReport> {
    let mut stream = TcpStream::connect_timeout(addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;


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
        onlines: json["players"]["online"].as_i64(),
        mods: read_forge_mods(&json)?,
        elapsed: elapsed,
        json: response.content,
    })
}

pub fn seek_send_bytes(host: &str) -> usize {
    let hadnshake_packet_len = 1 + 1 + get_varint_length(host.len()) + host.len() + 2 + 1;
    get_varint_length(hadnshake_packet_len) + hadnshake_packet_len + 2 + 10
}


// ref: src/main/java/net/minecraftforge/network/ServerStatusPing.java (1.20.x)
fn read_forge_mods(json: &Value) -> Result<Option<usize>> {
    if let Some(d) = json.get("forgeData").and_then(|m| m.get("d")) {
        let units: Vec<u16> = d.as_str().unwrap_or_default().encode_utf16().collect();
        if units.len() < 3 {
            return Ok(None);
        }
        
        let mut d  = &decode_forge_d(&units)?[1..];
        let size = d.read_unsigned_short()? as usize;
        Ok(Some(size))
    }
    else {
        Ok(json
            .get("modinfo")
            .and_then(|m| m.get("modList"))
            .and_then(|list| list.as_array())
            .map(|arr| arr.len()))        
    }
}
