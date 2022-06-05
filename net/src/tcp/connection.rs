use crate::ProtoMsgType;
use crate::{ProtoReceiver, ProtoSender};
use proto::allptos;
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{
    broadcast,
    mpsc::{self, error::TrySendError},
    Semaphore,
};

extern crate llog;

//一个完整的协议头部,包括: 协议id(u32) + 协议包总长度(u32), 满足8个字节
const PROTO_HEADER_LEN: usize = 8;
const INIT_PROTO_TOTAL_LEN: usize = 1024;
const PROTO_BODY_MAX_LEN: usize = 64 * 1024 - PROTO_HEADER_LEN;

#[derive(Debug)]
pub struct ConnReader {
    vfd: u64,
    stream: BufReader<OwnedReadHalf>,
    proto_tx: ProtoSender, // tcp msg send to outer service
    limit_connections: Arc<Semaphore>,
    _shutdown_complete: mpsc::Sender<()>,
    shutdown: bool,
    buffer: Vec<u8>,
    proto_id: u32,
    proto_len: usize,
    is_header_decode: bool,
    readnum: u64,
}

impl Drop for ConnReader {
    fn drop(&mut self) {
        self.limit_connections.add_permits(1);
    }
}

impl ConnReader {
    pub fn new(
        vfd: u64,
        stream: OwnedReadHalf,
        proto_tx: ProtoSender,
        limit_connections: Arc<Semaphore>,
        _shutdown_complete: mpsc::Sender<()>,
    ) -> ConnReader {
        ConnReader {
            vfd,
            stream: BufReader::new(stream),
            proto_tx,
            limit_connections,
            _shutdown_complete,
            shutdown: false,
            buffer: Vec::with_capacity(INIT_PROTO_TOTAL_LEN),
            proto_id: 0,
            proto_len: 0,
            is_header_decode: false,
            readnum: 0,
        }
    }

    pub async fn run(
        &mut self,
        log_name: &'static str,
        mut notify: broadcast::Receiver<()>,
    ) -> crate::Result<()> {
        while !self.shutdown {
            tokio::select! {
                res = self.read_frame() => {
                    if let Some(pto) = res? {
                        println!("recv: vfd={},proto_id={},readnum={}",pto.0,pto.1,self.readnum);

                        // 注意, 如果这里使用 send 发送会产生阻塞,而对端的消息处理完毕后也可能会有消息返回也是通过 send.
                        // 如果这边的 send 出现阻塞, 对端返回的 send 也同样出现阻塞, 这时候会导致两端的协程产生 deadlock.
                        // 解决的办法有 1) send 一个 oneshot 或者 2) 用 try_send 代替 send;
                        // 用 1) 的弊端是必须在 send 的一端等待消息返回. 而用 try_send 的弊端则是发送失败时,只能选择丢弃消息
                        // 这里选择 2), 因为这样做更符合背压原理, 对于游戏玩家的请求, 处理不过来就丢弃这也是合理的;
                        // 但对于 rpc 的服务类型,如果确保不了发送消息的成功就会出现麻烦事.
                        // :TODO: 对于 rpc 的发送, 后续是通过 spawn 一个协程来发送呢,还是有其他更好的办法.
                        // 这里暂时的做法是把未发送成功的协议记录下来,通过日志的错误提示,再寻求扩大队列还是其他更好的办法.

                        //self.proto_tx.send(pto).await?; // would block
                        if let Err(err) = self.proto_tx.try_send(pto) {
                            match err {
                                TrySendError::Full(err) => {
                                    llog::error!(log_name,"[ConnReader]: proto_tx send failed: vfd={},proto_id={}",err.0,err.1);
                                },
                                TrySendError::Closed(_err) =>{
                                    llog::error!(log_name,"[ConnReader]: proto_tx close: vfd={}",self.vfd);
                                    self.shutdown = true;
                                    break;
                                }
                            }
                        }
                    } else {
                        llog::info!(log_name,"[ConnReader]: tcp connection close: vfd={}",self.vfd);
                        self.shutdown = true;
                        break;
                    }
                }
                _ = notify.recv() => {
                    llog::info!(log_name,"[ConnReader]: notify connection close: vfd={}",self.vfd);
                    self.shutdown = true;
                    break;
                },
            };
        }
        Ok(())
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
            // :TODO: the capacity of this buffer will grow..when to shrink the buffer?
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
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
}

#[derive(Debug)]
pub struct ConnWriter {
    vfd: u64,
    stream: BufWriter<OwnedWriteHalf>,
    proto_rx: ProtoReceiver,
    writenum: u64,
}

impl ConnWriter {
    pub fn new(vfd: u64, stream: OwnedWriteHalf, proto_rx: ProtoReceiver) -> ConnWriter {
        ConnWriter {
            vfd,
            stream: BufWriter::new(stream),
            proto_rx,
            writenum: 0,
        }
    }

    pub async fn run<'a>(&mut self, log_name: &'a str) -> crate::Result<()> {
        while let Some((from_vfd, proto_id, pto)) = self.proto_rx.recv().await {
            if self.vfd != from_vfd {
                llog::info!(
                    log_name,
                    "[ConnWriter]: wrong vfd={}, from_vfd={}",
                    self.vfd,
                    from_vfd
                );
                break;
            }
            let buf = allptos::serialize(pto)?;
            self.write_frame(proto_id, &buf).await?;
        }
        llog::error!(log_name, "[ConnWriter]: closed: {}", self.vfd);
        Ok(())
    }

    pub async fn write_frame(&mut self, proto_id: u32, buf: &[u8]) -> io::Result<()> {
        self.writenum += 1;
        println!(
            "[write_frame]: vfd={},proto_id={},writenum={}",
            self.vfd, proto_id, self.writenum
        );

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
