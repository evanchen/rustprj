use crate::{ChanProtoSender, MailBox, ProtoSender};
use std::future::Future;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{
    broadcast,
    mpsc::{self, error::TrySendError},
    Semaphore,
};
use tokio::time::{self, Duration};

extern crate llog;

use crate::{Connection, ProtoMsgType, RecvType, ServiceType, Shutdown};
use proto::allptos;

// tcp socket 最大连接数量上限
const MAX_CONNECTIONS: usize = 10000;

#[derive(Debug)]
pub struct Listener {
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

#[derive(Debug)]
struct Handler {
    vfd: u64,
    connection: Connection,
    outsender: ProtoSender,
    mailbox: MailBox<ProtoMsgType>,
    limit_connections: Arc<Semaphore>,
    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
}

pub async fn run(
    serv_type: ServiceType,
    log_name: &'static str,
    listener: TcpListener,
    shutdown: impl Future,
    chan_out: ChanProtoSender,
    out_sender: ProtoSender,
) {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);
    let mut listener_obj = Listener {
        listener,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx,
    };

    tokio::select! {
        res = listener_obj.run(serv_type,log_name,chan_out,out_sender) => {
            if let Err(err) = res {
                llog::error!(log_name,"[listener.run]: error: {:?}",err);
            }
        }
        _ = shutdown => {
            llog::info!(log_name,"listener shut down.");
        }
    }

    let Listener {
        mut shutdown_complete_rx,
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = listener_obj;

    // only one sender is drop, receivers will get a closed error
    // then shutdown signal is finished.
    drop(notify_shutdown);
    // when connection receives a shutdown signal, connection itself will be dropped
    // so does its "_shutdown_complete".
    drop(shutdown_complete_tx);
    //wait for all other connections to be dropped.
    let _ = shutdown_complete_rx.recv().await;
}

impl Listener {
    async fn run(
        &mut self,
        serv_type: ServiceType,
        log_name: &'static str,
        chan_out: ChanProtoSender,
        out_sender: ProtoSender,
    ) -> crate::Result<()> {
        llog::info!(log_name, "[run]: accepting connections");

        let mut vfd = match serv_type {
            ServiceType::Tcp => 10000u64, // 来自游戏客户端的连接,vfd 从 10000 开始标识
            ServiceType::Rpc => 0u64,
        };
        loop {
            //don't ever close the sempahore, so `unwrap()` is safe.
            //add permit when the connection is closed.
            self.limit_connections.acquire().await.unwrap().forget();
            vfd += 1;

            let socket = self.accept(log_name).await?;
            let mut handler = Handler {
                vfd,
                connection: Connection::new(socket, vfd),
                outsender: out_sender.clone(), //把外界的sender传clone给自己
                mailbox: MailBox::new(100),
                limit_connections: self.limit_connections.clone(),
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                _shutdown_complete: self.shutdown_complete_tx.clone(),
            };

            //把自己的 sender 暴露给外界
            //这里的 send 不需要考虑背压,而且必须让对端接收了之后才继续运行 handler
            chan_out.send((vfd, handler.mailbox.send.clone())).await?;

            tokio::spawn(async move {
                if let Err(err) = handler.run(log_name).await {
                    llog::error!(log_name, "[handler.run]: error: {:?}", err);
                } else {
                    llog::info!(log_name, "connection handler.run() return");
                }
            });
        }
    }

    async fn accept(&mut self, log_name: &'static str) -> crate::Result<TcpStream> {
        let mut backoff = 1;
        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > 64 {
                        return Err(err.into());
                    }
                }
            }
            time::sleep(Duration::from_secs(backoff)).await;
            backoff *= 2;
            llog::info!(log_name, "[listener.accept]: backoff: {}", backoff);
        }
    }
}

impl Handler {
    async fn run(&mut self, log_name: &'static str) -> crate::Result<()> {
        while !self.shutdown.is_shutdown() {
            let (rtype, maybe_proto) = tokio::select! {
                res = self.connection.read_frame() => {
                    //收到 socket 接收的协议, 转发到 service 处理
                    (RecvType::FromSocket,res?)
                },
                res = self.mailbox.recv() => {
                    //收到 service 的协议, 通过 socket 发送给 client
                    (RecvType::FromService,res)
                }
                _ = self.shutdown.recv() => return Ok(()),
            };
            let pto = match maybe_proto {
                Some(pto) => pto,
                None => return Ok(()),
            };
            match rtype {
                RecvType::FromSocket => {
                    println!("server receive proto from socket,and transfer the proto to service: vfd={},proto_id={}",pto.0,pto.1);
                    // 对于网络连接中,游戏客户端发过来的消息,如果发送失败,可以直接丢弃,当作客户端网络丢包.
                    // :TODO: 但如果对于 rpc 调用的话, 特别是通过 rpc 发送到 db 进程进行数据存档的, 这里的确需要特别处理, 因为存档不能丢弃
                    if let Err(err) = self.outsender.try_send(pto) {
                        match err {
                            TrySendError::Full(err) => {
                                llog::error!(log_name,"server receive proto from socket,and transfer the proto to service, chan full: vfd={},proto_id={}",err.0,err.1);
                            }
                            TrySendError::Closed(_err) => {
                                return Err("outsender close".into());
                            }
                        }
                    }
                }
                RecvType::FromService => {
                    let vfd = pto.0;
                    if vfd != self.vfd {
                        llog::info!(
                            log_name,
                            "[handler.run]: wrong vfd={}, self.vfd={}",
                            vfd,
                            self.vfd
                        );
                        return Err("wrong vfd".into());
                    }
                    let proto_id = pto.1;
                    //println!("server send proto to socket: vfd={},proto_id={}",pto.0, pto.1);
                    let buf = allptos::serialize(pto.2)?;
                    self.connection.write_frame(proto_id, &buf).await?;
                }
            }
        }

        Ok(())
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.limit_connections.add_permits(1);
    }
}
