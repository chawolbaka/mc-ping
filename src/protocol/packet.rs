use std::io::{self, ErrorKind, Result, Write};

use crate::protocol::io::{MinecraftReadExt, MinecraftWriteExt};

// Altough packet id are defined as varint, but in Server List Ping, they only occupy 1 byte, so using u8 is suffcient.
const HANDSHAKE_PACKET_ID: u8 = 0;
const PING_REQUEST_PACKET_ID: u8 = 0;
const PING_RESPONSE_PACKET_ID: u8 = 0;
const PING_PACKET_ID: u8 = 1;
const PONG_PACKET_ID: u8 = 1;


pub trait Packet {
    // Why did I restrict it to Vec<u8> and &[u8]? Because using it with TcpStream is too slow.
    fn encode(&self) -> Result<Vec<u8>>;
    fn decode(packet: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Copy, Clone)]
pub enum HandshakeState {
    Status = 1,
    Login = 2,
    Transfer = 3,
}

pub struct HandshakePacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: HandshakeState,
}

pub struct PingRquestPacket {}

pub struct PingResponsePacket {
    pub content: String,
}

pub struct PingPacket {
    pub code: u64,
}

pub struct PongPacket {
    pub code: u64,
}

impl Packet for HandshakePacket {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut packet: Vec<u8> = Vec::new();
        packet.write_all(&[HANDSHAKE_PACKET_ID])?;
        packet.write_varint(self.protocol_version)?;
        packet.write_string(&self.server_address)?;
        packet.write_all(&self.server_port.to_be_bytes())?;
        packet.write_varint(self.next_state as i32)?;
        Ok(packet)
    }

    fn decode(packet: &[u8]) -> Result<Self> {
        check_id(packet, HANDSHAKE_PACKET_ID)?;
        let mut packet = &packet[1..];
        Ok(Self { 
            protocol_version: packet.read_varint()?,
            server_address: packet.read_string()?,
            server_port: packet.read_unsigned_short()?,
            next_state: match packet.read_varint()? {
                1 => HandshakeState::Status,
                2 => HandshakeState::Login,
                3 => HandshakeState::Transfer,
                other => return Err(io::Error::new(ErrorKind::InvalidData, format!("Invaild HandshakeState {other}"))),
            }
        })
    }
}

impl Packet for PingRquestPacket {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(vec![PING_REQUEST_PACKET_ID])
    }

    fn decode(packet: &[u8]) -> Result<Self> {
        check_id(packet, PING_REQUEST_PACKET_ID)?;
        Ok(Self {  })
    }
}

impl Packet for PingResponsePacket {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut packet: Vec<u8> = Vec::new();
        packet.write_all(&[PING_RESPONSE_PACKET_ID])?;
        packet.write_string(&self.content)?;
        Ok(packet)
    }

    fn decode(packet: &[u8]) -> Result<Self> {
        check_id(packet, PING_RESPONSE_PACKET_ID)?;
        let mut slice = &packet[1..];
        let content = slice.read_string()?;
        Ok(Self { content })
    }
}

impl Packet for PingPacket {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(encode_u64_packet(PING_PACKET_ID, self.code))
    }

    fn decode(packet: &[u8]) -> Result<Self> {
        Ok(Self { code: decode_u64_packet(packet, PING_PACKET_ID, "Ping")? })
    }
}

impl Packet for PongPacket {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(encode_u64_packet(PONG_PACKET_ID, self.code))
    }

    fn decode(packet: &[u8]) -> Result<Self> {
        Ok(Self { code: decode_u64_packet(packet, PONG_PACKET_ID, "Pong")? })
    }
}




#[inline]
fn check_id(packet: &[u8], id: u8) -> Result<()> {
    if packet.len() == 0 {
        return Err(io::Error::new(ErrorKind::InvalidData, "Empty packet"));
    }
    if packet[0] != id {
        return Err(io::Error::new(ErrorKind::InvalidData, "Invaild packet id"));
    }
    Ok(())
}

#[inline]
fn encode_u64_packet(id: u8, code: u64) -> Vec<u8> {
    let mut packet = [0u8; 9];
    packet[0] = id;
    packet[1..].copy_from_slice(&code.to_be_bytes());
    packet.to_vec()
}

#[inline]
fn decode_u64_packet(packet: &[u8], expected_id: u8, name: &'static str) -> Result<u64> {
    if packet.len() != 9 {
        return Err(io::Error::new(ErrorKind::InvalidData, format!("Invaild {name} packet length")));
    }

    check_id(packet, expected_id)?;
    Ok(u64::from_be_bytes(packet[1..9].try_into().unwrap()))
}