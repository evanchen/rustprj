// rpc 服务包括 tcp 套接字连接监听和数据发送.
// 当一个服务器开启 rpc 服务时,它就开始监听任意想跟它通信的 tcp 连接,
// 因此,这里的 rpc 服务跟游戏客户端进来的 tcp 连接没有区别.
//需要注意的是,
//  1). rpc 监听的套接字连接只接收信息,而不会给任何对端发送信息.
//  2). rpc 数据发送到对端时,会触发一个新的 tcp connection (如果不存在该 connection 的情况下).

use crate::{
    tcp, ChanProtoReceiver, ChanProtoSender, Communicate, MailBox, ProtoMsgType, ProtoSender,
    ServiceType,
};
use std::future::Future;
use tokio::sync::mpsc;

pub async fn start_service(
    addr: &str,
    shutdown: impl Future,
    chan_out_tx: ChanProtoSender,
    pto_out_sender: ProtoSender,
) {
    let serv_type = ServiceType::Rpc;
    let log_name = "rpc.log";
    tcp::tcp_service::start_service(
        serv_type,
        log_name,
        addr,
        shutdown,
        chan_out_tx,
        pto_out_sender,
    )
    .await;
}

pub async fn start_service_handler(
    chan_out_rx: ChanProtoReceiver,
    entity: impl Communicate<ProtoMsgType>,
    mailbox: MailBox<ProtoMsgType>,
    shutdown_notify_rx: mpsc::Receiver<()>,
) {
    let log_name = "rpc_handler.log";
    tcp::tcp_service::start_service_handler(
        log_name,
        chan_out_rx,
        entity,
        mailbox,
        shutdown_notify_rx,
    )
    .await;
}
