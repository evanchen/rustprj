//(from https://github.com/tafia/quick-protobuf.git)
//byte order is LittleEndian by default.

use std::convert::TryInto;

use crate::errors::{Error, Result};

#[derive(Debug, Clone)]
pub struct BytesReader {
    start: usize,
    end: usize,
}

impl BytesReader {
    pub fn new(start: usize, len: usize) -> BytesReader {
        BytesReader { start, end: len }
    }

    pub fn get_read_start(&self) -> usize {
        self.start
    }

    pub fn get_read_end(&self) -> usize {
        self.end
    }

    pub fn step(&mut self, size: usize) {
        self.start += size;
    }

    pub fn read_u8(&mut self, bytes: &[u8]) -> Result<u8> {
        let b = bytes.get(self.start).ok_or(Error::UnexpectedEndOfBuffer)?;
        self.start += 1;
        Ok(*b)
    }

    pub fn is_eof(&self) -> bool {
        self.start >= self.end
    }

    pub fn is_complete(&self) -> bool {
        self.start == self.end
    }

    pub fn get_len(&mut self, bytes: &[u8]) -> Result<usize> {
        match self.read_u32(bytes) {
            Ok(val) => Ok(val as usize),
            Err(err) => Err(err),
        }
    }

    // return (field_number << 3) | wire_type
    pub fn next_tag(&mut self, bytes: &[u8]) -> Result<u64> {
        self.read_u64(bytes)
    }

    pub fn read_i8(&mut self, bytes: &[u8]) -> Result<i8> {
        let b = self.read_u8(bytes)?;
        Ok(b as i8)
    }

    pub fn read_u16(&mut self, bytes: &[u8]) -> Result<u16> {
        let mut b = self.read_u8(bytes)?;
        if b & 0x80 == 0 {
            return Ok(b as u16);
        }
        let mut r = (b & 0x7f) as u32;

        b = self.read_u8(bytes)?;
        r |= ((b & 0x7f) as u32) << 7;
        if b & 0x80 == 0 {
            return Ok(r as u16);
        }

        b = self.read_u8(bytes)?;
        r |= ((b & 0x7f) as u32) << 14;
        if b & 0x80 == 0 {
            // 14 + 2 bit
            // 有最后一字节,那必须是1,否则报错吧
            return Ok(r as u16);
        }

        Err(Error::Varint("u16"))
    }

    pub fn read_i16(&mut self, bytes: &[u8]) -> Result<i16> {
        let r = self.read_u16(bytes)?;
        Ok(r as i16)
    }

    pub fn read_u32(&mut self, bytes: &[u8]) -> Result<u32> {
        let mut b = self.read_u8(bytes)?;
        if b & 0x80 == 0 {
            return Ok(b as u32);
        }
        let mut r = (b & 0x7f) as u32;

        b = self.read_u8(bytes)?;
        r |= ((b & 0x7f) as u32) << 7;
        if b & 0x80 == 0 {
            return Ok(r);
        }

        b = self.read_u8(bytes)?;
        r |= ((b & 0x7f) as u32) << 14;
        if b & 0x80 == 0 {
            return Ok(r);
        }

        b = self.read_u8(bytes)?;
        r |= ((b & 0x7f) as u32) << 21;
        if b & 0x80 == 0 {
            return Ok(r);
        }

        b = self.read_u8(bytes)?;
        r |= ((b & 0x7f) as u32) << 28;
        if b & 0x80 == 0 {
            // 28 + 4 bit
            return Ok(r);
        }

        Err(Error::Varint("u32"))
    }

    pub fn read_i32(&mut self, bytes: &[u8]) -> Result<i32> {
        let r = self.read_u32(bytes)?;
        Ok(r as i32)
    }

    pub fn read_u64(&mut self, bytes: &[u8]) -> Result<u64> {
        // part0
        let mut b = self.read_u8(bytes)?;
        if b & 0x80 == 0 {
            return Ok(b as u64);
        }
        let mut r0 = (b & 0x7f) as u32;

        b = self.read_u8(bytes)?;
        r0 |= ((b & 0x7f) as u32) << 7;
        if b & 0x80 == 0 {
            return Ok(r0 as u64);
        }

        b = self.read_u8(bytes)?;
        r0 |= ((b & 0x7f) as u32) << 14;
        if b & 0x80 == 0 {
            return Ok(r0 as u64);
        }

        b = self.read_u8(bytes)?;
        r0 |= ((b & 0x7f) as u32) << 21;
        if b & 0x80 == 0 {
            return Ok(r0 as u64);
        }

        // part1
        b = self.read_u8(bytes)?;
        let mut r1 = (b & 0x7f) as u32;
        if b & 0x80 == 0 {
            return Ok(r0 as u64 | (r1 as u64) << 28);
        }

        b = self.read_u8(bytes)?;
        r1 |= ((b & 0x7f) as u32) << 7;
        if b & 0x80 == 0 {
            return Ok(r0 as u64 | (r1 as u64) << 28);
        }

        b = self.read_u8(bytes)?;
        r1 |= ((b & 0x7f) as u32) << 14;
        if b & 0x80 == 0 {
            return Ok(r0 as u64 | (r1 as u64) << 28);
        }

        b = self.read_u8(bytes)?;
        r1 |= ((b & 0x7f) as u32) << 21;
        if b & 0x80 == 0 {
            return Ok(r0 as u64 | (r1 as u64) << 28);
        }

        // part 2
        b = self.read_u8(bytes)?;
        let mut r2 = (b & 0x7f) as u32;
        if b & 0x80 == 0 {
            return Ok((r0 as u64 | (r1 as u64) << 28) | (r2 as u64) << 56);
        }

        b = self.read_u8(bytes)?;
        r2 |= (b as u32) << 7;
        if b & 0x80 == 0 {
            return Ok((r0 as u64 | (r1 as u64) << 28) | (r2 as u64) << 56);
        }

        // cannot read more
        Err(Error::Varint("u64"))
    }

    pub fn read_i64(&mut self, bytes: &[u8]) -> Result<i64> {
        let r = self.read_u64(bytes)?;
        Ok(r as i64)
    }

    pub fn read_bool(&mut self, bytes: &[u8]) -> Result<bool> {
        let b = self.read_u8(bytes)?;
        Ok(b != 0)
    }

    //固定 4 bytes
    pub fn read_f32(&mut self, bytes: &[u8]) -> Result<f32> {
        let lebytes = bytes
            .get(self.start..self.start + 4)
            .ok_or(Error::UnexpectedEndOfBuffer)?;
        let val = f32::from_le_bytes(lebytes[..4].try_into().unwrap());
        self.start += 4;
        Ok(val)
    }

    //固定 8 bytes
    pub fn read_f64(&mut self, bytes: &[u8]) -> Result<f64> {
        let lebytes = bytes
            .get(self.start..self.start + 8)
            .ok_or(Error::UnexpectedEndOfBuffer)?;
        let val = f64::from_le_bytes(lebytes[..8].try_into().unwrap());
        self.start += 8;
        Ok(val)
    }

    pub fn read_string(&mut self, bytes: &[u8]) -> Result<String> {
        let len = self.get_len(bytes)?;
        let lebytes = bytes
            .get(self.start..self.start + len)
            .ok_or(Error::UnexpectedEndOfBuffer)
            .unwrap();
        let str = ::core::str::from_utf8(lebytes).unwrap();
        self.start += len;
        Ok(str.to_owned())
    }

    pub fn read_unknow(&mut self, _bytes: &[u8], tag: u64) -> Result<()> {
        Err(Error::UnknownWireType((tag & 0x7) as u8))
    }
}

pub trait MsgRead: Sized {
    fn read(r: &mut BytesReader, bytes: &[u8]) -> Result<Self>;
}
