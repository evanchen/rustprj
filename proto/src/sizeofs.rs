//! A module to compute the binary size of data once encoded (from https://github.com/tafia/quick-protobuf.git)
//!
//! This module is used primilarly when implementing the `MessageWrite::get_size`

/// Computes the binary size of the varint encoded u64
///
/// https://developers.google.com/protocol-buffers/docs/encoding
pub fn sizeof_varint(v: u64) -> usize {
    match v {
        0x0..=0x7F => 1,
        0x80..=0x3FFF => 2,
        0x4000..=0x1FFFFF => 3,
        0x200000..=0xFFFFFFF => 4,
        0x10000000..=0x7FFFFFFFF => 5,
        0x0800000000..=0x3FFFFFFFFFF => 6,
        0x040000000000..=0x1FFFFFFFFFFFF => 7,
        0x02000000000000..=0xFFFFFFFFFFFFFF => 8,
        0x0100000000000000..=0x7FFFFFFFFFFFFFFF => 9,
        _ => 10, //64位/7 = 9字节余1bit,所以需要至少10字节
    }
}

/// Computes the binary size of a variable length chunk of data (wire type 2)
///
/// The total size is the varint encoded length size plus the length itself
/// https://developers.google.com/protocol-buffers/docs/encoding
pub fn sizeof_len(len: usize) -> usize {
    //:TODO: this sizeof_len is just focus the "length" value itself.
    sizeof_varint(len as u64)
}

pub fn sizeof_tag(tag: u64) -> usize {
    sizeof_varint(tag)
}

/// Computes the binary size of the varint encoded u8
pub fn sizeof_u8(_: u8) -> usize {
    1
}

/// Computes the binary size of the varint encoded i8
pub fn sizeof_i8(_: i8) -> usize {
    1
}

/// Computes the binary size of the varint encoded u16
pub fn sizeof_u16(v: u16) -> usize {
    sizeof_varint(v as u64)
}

/// Computes the binary size of the varint encoded i16
pub fn sizeof_i16(v: i16) -> usize {
    sizeof_varint(v as u16 as u64)
}

/// Computes the binary size of the varint encoded u32
pub fn sizeof_u32(v: u32) -> usize {
    sizeof_varint(v as u64)
}

/// Computes the binary size of the varint encoded i32
pub fn sizeof_i32(v: i32) -> usize {
    sizeof_varint(v as u32 as u64)
}

/// Computes the binary size of the varint encoded u64
pub fn sizeof_u64(v: u64) -> usize {
    sizeof_varint(v)
}

/// Computes the binary size of the varint encoded i64
pub fn sizeof_i64(v: i64) -> usize {
    sizeof_varint(v as u64)
}

/// Computes the binary size of the varint encoded bool (always = 1)
pub fn sizeof_bool(_: bool) -> usize {
    1
}

/// Computes the binary size of the varint encoded f32
pub fn sizeof_f32(_v: f32) -> usize {
    4
}

/// Computes the binary size of the varint encoded f64
pub fn sizeof_f64(_v: f64) -> usize {
    8
}

/// Computes the binary size of the varint encoded string
pub fn sizeof_string(v: &str) -> usize {
    let len = v.len();
    len + sizeof_varint(len as u64)
}
