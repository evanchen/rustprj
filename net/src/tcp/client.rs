// 这个模块主要是用来辅助测试
use crate::{ChanProtoSender, ConnReader, ConnWriter, ProtoMsgType, ProtoSender};
use std::future::Future;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, Semaphore};

extern crate llog;
const LOG_NAME: &str = "tcp_client.log";

pub async fn run(
    addr: String,
    shutdown: impl Future,
    identity: u64,
    chan_out: ChanProtoSender,
    out_sender: ProtoSender,
) -> crate::Result<()> {
    let vfd = identity;
    let log_name = LOG_NAME;

    let stream = TcpStream::connect(addr).await.unwrap();
    let (conn_tx, conn_rx) = mpsc::channel::<ProtoMsgType>(100);
    let (read_stream, write_stream) = stream.into_split();

    let (notify_shutdown, _) = broadcast::channel(1);
    let limit_connections = Arc::new(Semaphore::new(1));
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

    let mut reader = ConnReader::new(
        vfd,
        read_stream,
        out_sender.clone(),
        limit_connections.clone(),
        shutdown_complete_tx.clone(),
    );

    let mut writer = ConnWriter::new(vfd, write_stream, conn_rx);

    // 暴露自己的消息输入端给外界
    chan_out.send((identity, conn_tx)).await?;

    // 开启 socket 消息写循环
    tokio::spawn(async move {
        if let Err(err) = writer.run(log_name).await {
            llog::error!(log_name, "[ConnWriter]: error: vfd={},{:?}", vfd, err);
        } else {
            llog::error!(log_name, "[ConnWriter]: return,vfd={}", vfd);
        }
    });

    // 开启socket 消息读循环
    let nofity = notify_shutdown.subscribe();
    tokio::spawn(async move {
        if let Err(err) = reader.run(log_name, nofity).await {
            llog::error!(log_name, "[ConnReader]: error: vfd={},{:?}", vfd, err);
        } else {
            llog::info!(log_name, "[ConnReader]: return,vfd={}", vfd);
        }
    });
    let _ = shutdown.await;

    // only one sender is drop, receivers will get a closed error
    // then shutdown signal is finished.
    drop(notify_shutdown);
    // when connection receives a shutdown signal, connection itself will be dropped
    // so does its "_shutdown_complete".
    drop(shutdown_complete_tx);
    //wait for all other connections to be dropped.
    let _ = shutdown_complete_rx.recv().await;

    Ok(())
}
