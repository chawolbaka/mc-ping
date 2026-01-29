use std::io::{self, ErrorKind, Read, Result, Write};

impl<R: Read + ?Sized> MinecraftReadExt for R {}
impl<W: Write + ?Sized> MinecraftWriteExt for W {}

pub trait MinecraftReadExt: Read {
    fn read_string(&mut self) -> Result<String> {
        let len = self.read_varint()? as usize;
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        let value =
            String::from_utf8(buf).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        Ok(value)
    }
    fn read_unsigned_short(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_varint(&mut self) -> Result<i32> {
        let mut number: i32 = 0;
        let mut buf = [0u8; 1];
        for i in 0..5 {
            self.read_exact(&mut buf)?;
            let byte: i32 = buf[0] as i32;
            number |= (byte & 0b0111_1111) << i * 7;
            if byte & 0b1000_0000 == 0 {
                return Ok(number);
            }
        }
        Err(io::Error::new(ErrorKind::InvalidData, "varint overflow"))
    }

    fn read_packet(&mut self) -> Result<Vec<u8>> {
        let len = self.read_varint()? as usize;
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
}

pub trait MinecraftWriteExt: Write {
    fn write_string(&mut self, str: &str) -> Result<()> {
        self.write_varint(str.len() as i32)?;
        self.write_all(str.as_bytes())?;
        Ok(())
    }

    fn write_varint(&mut self, mut number: i32) -> Result<()> {
        while number & -128 != 0 {
            let byte = (number as u8) & 0b0111_1111 | 0b1000_0000;
            self.write_all(&[byte])?;
            number = ((number as u32) >> 7) as i32;
        }
        self.write_all(&[number as u8])?;
        Ok(())
    }

    fn write_packet(&mut self, packet: &[u8]) -> Result<()> {
        // Writing data directly to a TCP stream can be slow, so it should be buffered first.
        let mut buf: Vec<u8> = Vec::new();
        buf.write_varint(packet.len() as i32)?;
        buf.write_all(packet)?;
        self.write_all(&buf)?;
        Ok(())
    }
}

pub fn seek_varint_length(value: usize) -> usize {
    let temp = value as u32;
    if (temp & 0xFFFF_FF80) == 0 {
        1
    } else if (temp >> 14) == 0 {
        2
    } else if (temp >> 21) == 0 {
        3
    } else if (temp >> 28) == 0 {
        4
    } else {
        5
    }
}

// Decode Forge's packed "d" field into raw bytes.
pub fn decode_forge_d(units: &[u16]) -> Result<Vec<u8>> {
    if units.len() < 2 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "input too short: need at least 2 UTF-16 code units for size header",
        ));
    }

    let size = (u32::from(units[0]) | (u32::from(units[1]) << 15)) as usize;

    let mut out = Vec::new();
    out.try_reserve(size).map_err(|_| {
        io::Error::new(ErrorKind::OutOfMemory, "unable to allocate output buffer")
    })?;

    let mut buffer: u32 = 0;
    let mut bits_in_buf: i32 = 0;

    // Forge encodes bytes in UTF-16 units using 15-bit chunks.
    for &unit in &units[2..] {
        while bits_in_buf >= 8 {
            out.push((buffer & 0xFF) as u8);
            buffer >>= 8;
            bits_in_buf -= 8;
        }

        buffer |= (u32::from(unit) & 0x7FFF) << (bits_in_buf as u32);
        bits_in_buf += 15;
    }

    while out.len() < size {
        out.push((buffer & 0xFF) as u8);
        buffer >>= 8;
        // bits_in_buf -= 8;
    }

    Ok(out)
}
