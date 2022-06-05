use crate::{ChanProtoSender, ProtoSender};
use std::future::Future;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time::{self, Duration};
extern crate llog;
use crate::{ConnReader, ConnWriter, ProtoMsgType, ServiceType};

// tcp socket 最大连接数量上限
const MAX_CONNECTIONS: usize = 10000;

#[derive(Debug)]
pub struct Listener {
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
    counter: u64,
    serv_type: ServiceType,
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
    let mut listener = Listener {
        listener,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx,
        counter: 0,
        serv_type,
    };
    let base_counter = match listener.serv_type {
        ServiceType::Tcp => 10000u64, // 来自游戏客户端的连接,vfd 从 10000 开始标识
        ServiceType::Rpc => 0u64,
    };
    listener.counter = base_counter;

    tokio::select! {
        res = listener.run(log_name,chan_out,out_sender) => {
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
    } = listener;

    drop(notify_shutdown);
    drop(shutdown_complete_tx);
    let _ = shutdown_complete_rx.recv().await;
}

impl Listener {
    async fn run(
        &mut self,
        log_name: &'static str,
        chan_out: ChanProtoSender,
        out_sender: ProtoSender,
    ) -> crate::Result<()> {
        llog::info!(log_name, "[run]: accepting connections");

        loop {
            // 给每个新连接一个自增的id
            self.counter += 1;
            let vfd = self.counter;
            let stream = self.accept(log_name).await?;
            let (read_stream, write_stream) = stream.into_split();
            let mut reader = ConnReader::new(
                vfd,
                read_stream,
                out_sender.clone(),
                self.limit_connections.clone(),
                self.shutdown_complete_tx.clone(),
            );

            // 根据服务类型决定 channel 队列大小
            let ch_bound_size = match self.serv_type {
                ServiceType::Tcp => 100,
                ServiceType::Rpc => 2000,
            };
            let (conn_tx, conn_rx) = mpsc::channel::<ProtoMsgType>(ch_bound_size);
            let mut writer = ConnWriter::new(vfd, write_stream, conn_rx);

            // 在 reader 被 drop 时归还计数
            self.limit_connections.acquire().await.unwrap().forget();

            // 暴露自己的消息输入端给外界, :TODO: 注意这里会产生阻塞
            chan_out.send((vfd, conn_tx)).await?;

            // 开启 socket 消息写循环
            tokio::spawn(async move {
                if let Err(err) = writer.run(log_name).await {
                    llog::error!(log_name, "[ConnWriter]: error: vfd={},{:?}", vfd, err);
                } else {
                    llog::info!(log_name, "[ConnWriter]: return,vfd={}", vfd);
                }
            });

            // 开启socket 消息读循环
            let nofity = self.notify_shutdown.subscribe();
            tokio::spawn(async move {
                if let Err(err) = reader.run(log_name, nofity).await {
                    llog::error!(log_name, "[ConnReader]: error: vfd={},{:?}", vfd, err);
                } else {
                    llog::info!(log_name, "[ConnReader]: return,vfd={}", vfd);
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
