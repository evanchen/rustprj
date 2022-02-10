//(from https://github.com/tafia/quick-protobuf.git)
//byte order is LittleEndian by default.

use crate::errors::{Error, Result};

#[derive(Debug)]
pub struct BytesWriter<'a> {
    cursor: usize,
    buf: &'a mut Vec<u8>,
}

impl<'a> BytesWriter<'a> {
    pub fn new(buf: &'a mut Vec<u8>) -> BytesWriter<'a> {
        BytesWriter { cursor: 0, buf }
    }

    pub fn get_write_pos(&self) -> usize {
        self.cursor
    }

    pub fn write_u8(&mut self, val: u8) -> Result<()> {
        let total = self.cursor + 1;
        if total > self.buf.capacity() {
            return Err(Error::OutputBufferTooSmall(
                self.cursor,
                1,
                self.buf.capacity(),
            ));
        }
        self.buf.push(val);
        self.cursor = total;
        Ok(())
    }

    pub fn write_vint(&mut self, val: u64, maxbytes: i32) -> Result<()> {
        let mut val = val;
        let mut maxbytes = maxbytes;
        while val > 0x7f {
            self.write_u8(((val as u8) & 0x7f) | 0x80)?;
            val >>= 7;
            maxbytes -= 1;
        }
        assert!(maxbytes >= 0);
        self.write_u8(val as u8)
    }

    // (field_number << 3) | wire_type
    pub fn write_tag(&mut self, tag: u64) -> Result<()> {
        self.write_u64(tag)
    }

    pub fn write_len(&mut self, val: usize) -> Result<()> {
        if val >= u32::MAX as usize {
            return Err(Error::Message("len beyond max".to_owned()));
        }
        self.write_vint(val as u32 as u64, 4)
    }

    pub fn write_i8(&mut self, val: i8) -> Result<()> {
        self.write_u8(val as u8)
    }

    pub fn write_u16(&mut self, val: u16) -> Result<()> {
        self.write_vint(val as u64, 2)
    }

    pub fn write_i16(&mut self, val: i16) -> Result<()> {
        self.write_vint(val as u16 as u64, 2) //不能直接转换成u64,因为是负数时,as 转换会把符号位转换成高位的1
    }

    pub fn write_u32(&mut self, val: u32) -> Result<()> {
        self.write_vint(val as u64, 4)
    }

    pub fn write_i32(&mut self, val: i32) -> Result<()> {
        self.write_vint(val as u32 as u64, 4)
    }

    pub fn write_u64(&mut self, val: u64) -> Result<()> {
        self.write_vint(val, 9)
    }

    pub fn write_i64(&mut self, val: i64) -> Result<()> {
        self.write_vint(val as u64, 9)
    }

    pub fn write_bool(&mut self, val: bool) -> Result<()> {
        let val = if val { 1 } else { 0 };
        self.write_u8(val)
    }

    //固定 4 bytes
    pub fn write_f32(&mut self, val: f32) -> Result<()> {
        let lebytes = f32::to_le_bytes(val);
        let len = lebytes.len();
        assert!(len == 4);
        let total = self.cursor + len;
        if total > self.buf.capacity() {
            return Err(Error::OutputBufferTooSmall(
                self.cursor,
                len,
                self.buf.capacity(),
            ));
        }
        //self.buf[self.cursor..total].copy_from_slice(&lebytes);
        for v in lebytes {
            self.buf.push(v);
        }
        self.cursor = total;
        Ok(())
    }

    //固定 8 bytes
    pub fn write_f64(&mut self, val: f64) -> Result<()> {
        let lebytes = f64::to_le_bytes(val);
        let len = lebytes.len();
        assert!(len == 8);
        let total = self.cursor + len;
        if total > self.buf.capacity() {
            return Err(Error::OutputBufferTooSmall(
                self.cursor,
                len,
                self.buf.capacity(),
            ));
        }
        //self.buf[self.cursor..total].copy_from_slice(&lebytes);
        for v in lebytes {
            self.buf.push(v);
        }
        self.cursor = total;
        Ok(())
    }

    pub fn write_string(&mut self, val: &str) -> Result<()> {
        let len = val.len();
        self.write_len(len)?;
        let total = self.cursor + len;
        if total > self.buf.capacity() {
            return Err(Error::OutputBufferTooSmall(
                self.cursor,
                len,
                self.buf.capacity(),
            ));
        }
        //self.buf[self.cursor..total].copy_from_slice(&lebytes);
        for v in val.as_bytes() {
            self.buf.push(*v);
        }
        self.cursor = total;
        Ok(())
    }

    pub fn write_u8_with_tag(&mut self, tag: u64, val: u8) -> Result<()> {
        self.write_tag(tag)?;
        self.write_u8(val)
    }
    pub fn write_i8_with_tag(&mut self, tag: u64, val: i8) -> Result<()> {
        self.write_tag(tag)?;
        self.write_i8(val)
    }
    pub fn write_u16_with_tag(&mut self, tag: u64, val: u16) -> Result<()> {
        self.write_tag(tag)?;
        self.write_u16(val)
    }
    pub fn write_i16_with_tag(&mut self, tag: u64, val: i16) -> Result<()> {
        self.write_tag(tag)?;
        self.write_i16(val)
    }
    pub fn write_u32_with_tag(&mut self, tag: u64, val: u32) -> Result<()> {
        self.write_tag(tag)?;
        self.write_u32(val)
    }
    pub fn write_i32_with_tag(&mut self, tag: u64, val: i32) -> Result<()> {
        self.write_tag(tag)?;
        self.write_i32(val)
    }
    pub fn write_u64_with_tag(&mut self, tag: u64, val: u64) -> Result<()> {
        self.write_tag(tag)?;
        self.write_u64(val)
    }
    pub fn write_i64_with_tag(&mut self, tag: u64, val: i64) -> Result<()> {
        self.write_tag(tag)?;
        self.write_i64(val)
    }
    pub fn write_bool_with_tag(&mut self, tag: u64, val: bool) -> Result<()> {
        self.write_tag(tag)?;
        self.write_bool(val)
    }
    pub fn write_f32_with_tag(&mut self, tag: u64, val: f32) -> Result<()> {
        self.write_tag(tag)?;
        self.write_f32(val)
    }
    pub fn write_f64_with_tag(&mut self, tag: u64, val: f64) -> Result<()> {
        self.write_tag(tag)?;
        self.write_f64(val)
    }
    pub fn write_string_with_tag(&mut self, tag: u64, val: &str) -> Result<()> {
        self.write_tag(tag)?;
        self.write_string(val)
    }
}

pub trait MsgWrite {
    fn size(&self) -> usize;
    fn write(&self, w: &mut BytesWriter) -> Result<()>;
}
