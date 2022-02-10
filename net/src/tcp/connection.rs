use crate::ProtoMsgType;
use proto::allptos;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

//一个完整的协议头部,包括: 协议id(u32) + 协议包总长度(u32), 满足8个字节
const PROTO_HEADER_LEN: usize = 8;
const PROTO_TOTAL_LEN: usize = 64 * 1024;
const PROTO_BODY_MAX_LEN: usize = PROTO_TOTAL_LEN - PROTO_HEADER_LEN;

#[derive(Debug)]
pub struct Connection {
    vfd: u64,
    stream: BufWriter<TcpStream>,
    buffer: Vec<u8>,
    proto_id: u32,
    proto_len: usize,
    is_header_decode: bool,
    readnum: u64,
    writenum: u64,
}

impl Connection {
    pub fn new(socket: TcpStream, vfd: u64) -> Connection {
        Connection {
            vfd,
            stream: BufWriter::new(socket),
            buffer: Vec::with_capacity(PROTO_TOTAL_LEN),
            proto_id: 0,
            proto_len: 0,
            is_header_decode: false,
            readnum: 0,
            writenum: 0,
        }
    }

    pub async fn read_frame(&mut self) -> crate::Result<Option<ProtoMsgType>> {
        loop {
            if let Some(pto) = self.parse_frame()? {
                self.readnum += 1;
                //println!("[read_frame]: readnum={},proto_id={}",self.readnum,self.proto_id);
                self.is_header_decode = false;
                self.proto_id = 0;
                self.proto_len = 0;
                return Ok(Some(pto));
            }
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                // The remote closed the connection. For this to be a clean
                // shutdown, there should be no data in the read buffer. If
                // there is, this means that the peer closed the socket while
                // sending a frame.

                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    // return Ok(None) means continue read from stream.
    fn parse_frame(&mut self) -> crate::Result<Option<ProtoMsgType>> {
        //缓存里还剩余的数据长度
        let buflen = self.buffer.len();
        // buffer 必须满足一整个协议包的内容空间大小之后才开始协议对象的序列化
        // protoid(4bytes) + packlen(4bytes) + body(packlen bytes)
        if buflen < PROTO_HEADER_LEN {
            return Ok(None);
        }
        if !self.is_header_decode {
            let mut proto_id = 0u32;
            proto_id |= self.buffer[0] as u32 & 0xff;
            proto_id |= (self.buffer[1] as u32 & 0xff) << 8;
            proto_id |= (self.buffer[2] as u32 & 0xff) << 16;
            proto_id |= (self.buffer[3] as u32 & 0xff) << 24;

            let mut proto_len = 0u32;
            proto_len |= self.buffer[4] as u32 & 0xff;
            proto_len |= (self.buffer[5] as u32 & 0xff) << 8;
            proto_len |= (self.buffer[6] as u32 & 0xff) << 16;
            proto_len |= (self.buffer[7] as u32 & 0xff) << 24;

            // for i in 0..4 {
            //     proto_id |= ((self.buffer[i] as u32) & 0xff) << (i*8);
            //     println!("r parse proto_id: {},{},{}",i,(self.buffer[i]),proto_id);
            // }
            // for i in 4..PROTO_HEADER_LEN {
            //     proto_len |= ((self.buffer[i] as u32) & 0xff) << (i - 4);
            //     println!("r parse proto_len: {},{},{}",i,(self.buffer[i]),proto_len);
            // }
            self.proto_id = proto_id;
            self.proto_len = proto_len as usize;
            self.is_header_decode = true;
        }
        //协议长度超出最大上限
        if self.proto_len >= PROTO_BODY_MAX_LEN {
            return Err(
                format!("[parse_fram]: exceed PROTO_BODY_MAX_LEN,{}", self.proto_len).into(),
            );
        }
        let protolen = self.proto_len + PROTO_HEADER_LEN;
        //剩余缓存数据长度还未满足协议数据所需长度,我们认为是接收字节流未完成
        if buflen < protolen {
            return Ok(None);
        }
        //println!("proto_id={},protolen={},buflen={},bufcap={},header={:?}",self.proto_id,self.proto_len,buflen,self.buffer.capacity(),&self.buffer[0..PROTO_HEADER_LEN]);
        match allptos::parse_proto(self.proto_id, &self.buffer, PROTO_HEADER_LEN, protolen) {
            Ok(ptoobj) => {
                //把 buffer 剩余的内容往前拷贝
                let leftlen = buflen - protolen;
                if leftlen != 0 {
                    // 等于0就不用拷贝了
                    self.buffer.copy_within(protolen..buflen, 0);
                }
                //设置当前 len
                unsafe {
                    self.buffer.set_len(leftlen);
                };
                Ok(Some((self.vfd, self.proto_id, ptoobj)))
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Write a single `Frame` value to the underlying stream.
    ///
    /// The `Frame` value is written to the socket using the various `write_*`
    /// functions provided by `AsyncWrite`. Calling these functions directly on
    /// a `TcpStream` is **not** advised, as this will result in a large number of
    /// syscalls. However, it is fine to call these functions on a *buffered*
    /// write stream. The data will be written to the buffer. Once the buffer is
    /// full, it is flushed to the underlying socket.
    pub async fn write_frame(&mut self, proto_id: u32, buf: &[u8]) -> io::Result<()> {
        self.writenum += 1;
        //println!("[write_frame]: writenum={},proto_id={}",self.writenum,proto_id);

        let buflen = buf.len() as u32;
        // little-endian
        let mut header = 0u64;
        header |= proto_id as u64;
        header |= ((buflen as u64) & 0xffffffff) << 32;

        // let mut header2 = [0u8;PROTO_HEADER_LEN];
        // header2[0] = (proto_id & 0xff) as u8;
        // header2[1] = ((proto_id >> 8) & 0xff) as u8;
        // header2[2] = ((proto_id >> 16) & 0xff) as u8;
        // header2[3] = ((proto_id >> 24) & 0xff) as u8;

        // header2[4] = (buflen & 0xff) as u8;
        // header2[5] = ((buflen >> 8) & 0xff) as u8;
        // header2[6] = ((buflen >> 16) & 0xff) as u8;
        // header2[7] = ((buflen >> 24) & 0xff) as u8;
        // println!("write_frame: proto_id={},buflen={}, header.bytes={:?},header2={:?}",proto_id,buflen,header.to_le_bytes(),header2);
        //self.stream.write(&header2).await?;

        self.stream.write_u64_le(header).await?;
        self.stream.write_all(&buf).await?;

        // Ensure the encoded frame is written to the socket. The calls above
        // are to the buffered stream and writes. Calling `flush` writes the
        // remaining contents of the buffer to the socket.
        self.stream.flush().await
    }
}
