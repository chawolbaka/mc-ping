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


// 该方法完全由AI编写，涉及java和rust对位运算可能存在的差异，我对自己的水平没有信心（顺便甩锅，出现问题了都是AI的错！）
pub fn decode_forge_d(units: &[u16]) -> Result<Vec<u8>> {
    if units.len() < 2 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "input too short: need at least 2 UTF-16 code units for size header",
        ));
    }

    // size = size0 | (size1 << 15)
    let size0 = units[0] as u32;
    let size1 = units[1] as u32;
    let size = (size0 | (size1 << 15)) as usize;

    // 等价于 Unpooled.buffer(size): 初始容量 size，但允许继续增长
    let mut out = Vec::<u8>::new();
    out.try_reserve(size).map_err(|_| {
        io::Error::new(ErrorKind::OutOfMemory, "unable to allocate output buffer")
    })?;

    let mut string_index: usize = 2;
    let mut buffer: u32 = 0;     // Java 注释：最多 ~22 bits，u32 足够
    let mut bits_in_buf: i32 = 0;

    while string_index < units.len() {
        // 与 Java 完全一致：只要 bitsInBuf >= 8 就持续 writeByte，
        // 不检查是否超过 size（因此可能 out.len() > size）
        while bits_in_buf >= 8 {
            out.push((buffer & 0xFF) as u8);
            buffer >>= 8;        // 等价 Java >>>= 8
            bits_in_buf -= 8;
        }

        let c = units[string_index] as u32;
        // 此处 bits_in_buf 理论上在 [0,7]，移位安全
        buffer |= (c & 0x7FFF) << (bits_in_buf as u32);
        bits_in_buf += 15;
        string_index += 1;
    }

    // write any leftovers: 仅当当前可读字节数 < size 时，继续写到 size
    // 若主循环已写出 > size，则这里不会截断（严格模拟 ByteBuf 逻辑）
    while out.len() < size {
        out.push((buffer & 0xFF) as u8);
        buffer >>= 8;
        bits_in_buf -= 8; // 保持与 Java 一致（即便变负也无所谓，后续不再使用）
    }

    Ok(out)
}
