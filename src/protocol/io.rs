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
            let byte = (number as u8) & 0b1111111 | 0b1000_0000;
            self.write_all(&[byte])?;
            number = ((number as u32) >> 7) as i32;
        }
        self.write_all(&[number as u8])?;
        Ok(())
    }

    fn write_packet(&mut self, packet: &mut Vec<u8>) -> Result<()> {
        let len = packet.len();
        if len & 0b1000_0000 == 0 {
            packet.insert(0, len as u8);
        } else {
            todo!();
        }
        self.write_all(packet)?;
        Ok(())
    }
}


pub fn get_varint_length(value: usize) -> usize {
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