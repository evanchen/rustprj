use std::future::Future;
use tokio::net::TcpStream;

use crate::{ChanProtoSender, Connection, MailBox, ProtoMsgType, ProtoSender, RecvType};
use proto::allptos;

extern crate llog;
const LOG_NAME: &str = "tcp_client.log";

#[derive(Debug)]
pub struct Client {
    identity: u64,
    connection: Connection,
    outsender: ProtoSender,
    mailbox: MailBox<ProtoMsgType>,
}

pub async fn run(
    addr: String,
    shutdown: impl Future,
    identity: u64,
    chan_out: ChanProtoSender,
    out_sender: ProtoSender,
) -> crate::Result<()> {
    // Establish a connection
    let socket = TcpStream::connect(addr).await.unwrap();
    let mut client = Client {
        identity,
        connection: Connection::new(socket, identity),
        outsender: out_sender.clone(),
        mailbox: MailBox::new(1000),
    };

    //把自己的sender暴露给外界
    chan_out
        .send((identity, client.mailbox.send.clone()))
        .await?;

    tokio::select! {
        res = client.run() => {
            if let Err(err) = res {
                llog::error!(LOG_NAME,"[tcp_client.run]: error: {:?}",err);
                return Err(err);
            } else {
                println!("connection run() return");
            }
        }
        _ = shutdown => {
            llog::info!(LOG_NAME,"client shut down.");
        }
    }
    Ok(())
}

impl Client {
    pub async fn run(&mut self) -> crate::Result<()> {
        llog::info!(LOG_NAME, "[run]: accepting message");
        loop {
            let (rtype, maybe_proto) = tokio::select! {
                res = self.connection.read_frame() => {
                    //收到 socket 接收的协议, 转发到 service 处理
                    (RecvType::FromSocket,res?)
                },
                res = self.mailbox.recv() => {
                    //收到 service 的协议, 通过 socket 发送给 client
                    (RecvType::FromService,res)
                }
            };
            let pto = match maybe_proto {
                Some(pto) => pto,
                None => return Ok(()),
            };
            match rtype {
                RecvType::FromSocket => {
                    //println!("cli receive proto from socket,and transfer the proto to service: vfd={},proto_id={}",pto.0,pto.1);
                    self.outsender.send(pto).await?;
                }
                RecvType::FromService => {
                    let (identity, proto_id, pto) = pto;
                    if identity != self.identity {
                        llog::info!(
                            LOG_NAME,
                            "[handler.run]: wrong identity={}, self.identity={}",
                            identity,
                            self.identity
                        );
                        return Err("wrong identity".into());
                    }
                    //println!("cli send proto to socket: identity={},proto_id={}", identity,proto_id);
                    let buf = allptos::serialize(pto)?;
                    self.connection.write_frame(proto_id, &buf).await?;
                }
            }
        }
    }
}
