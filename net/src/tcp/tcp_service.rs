use crate::{
    tcp::listener, ChanProtoReceiver, ChanProtoSender, Communicate, MailBox, ProtoMsgType,
    ProtoSender, ServiceType,
};
use llog;
use std::future::Future;
use tokio::{
    net::TcpListener,
    sync::mpsc::{self, error::TrySendError},
    time::{self, Duration},
};

pub async fn start_service(
    serv_type: ServiceType,
    log_name: &'static str,
    addr: &str,
    shutdown: impl Future,
    chan_out_tx: ChanProtoSender,
    pto_out_sender: ProtoSender,
) {
    llog::info!(log_name, "service start: listening {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    listener::run(
        serv_type,
        log_name,
        listener,
        shutdown,
        chan_out_tx,
        pto_out_sender,
    )
    .await;
    llog::info!(log_name, "service stop");
}

pub async fn start_service_handler(
    log_name: &'static str,
    mut chan_out_rx: ChanProtoReceiver,
    mut entity: impl Communicate<ProtoMsgType>,
    mut mailbox: MailBox<ProtoMsgType>,
    mut shutdown_notify_rx: mpsc::Receiver<()>,
) {
    let mut heart_beat = time::interval(Duration::from_millis(1000));
    loop {
        tokio::select! {
            res = chan_out_rx.recv() => {
                if let Some((vfd,sender)) = res {
                    entity.register(vfd,sender);
                    llog::info!(log_name,"new client connection channel: vfd={}",vfd);
                } else {
                    llog::error!(log_name,"chan_out_rx channel broken");
                    break;
                }
            },
            res = mailbox.recv() => {
                if let Some((vfd,proto_id,pto)) = res {
                    //println!("service get proto: vfd={},proto_id={}",vfd,proto_id);
                    match entity.get(vfd) {
                        Some(ch) => {
                            // :TODO: 处理 pto

                            // 这是发送给 socket 的消息, 可以考虑用 try_send, 直接丢弃队列溢出的消息
                            if let Err(err) = ch.try_send((vfd,proto_id,pto)) {
                                match err {
                                    TrySendError::Full(err) => {
                                        llog::error!(log_name,"service send proto to connection failed, chan full: vfd={},proto_id={}",err.0,err.1);
                                    },
                                    TrySendError::Closed(_err) =>{
                                        llog::error!(log_name,"connection channel close: vfd={}",vfd);
                                        // should remove ch
                                        entity.unregister(vfd);
                                    }
                                }
                            }
                        },
                        None => {
                            // ch 不存在
                            llog::error!(log_name,"service send proto,connection doesn't exist: vfd={},proto_id={}",vfd,proto_id);
                        }
                    }
                } else {
                    llog::error!(log_name,"server service receive close");
                    break;
                }
            }
            _ = heart_beat.tick() => {
                println!("service heart_beat tick");
            }
            _ = shutdown_notify_rx.recv() => {
                llog::error!(log_name,"server service shutdown");
                break;
            }
        }
    }
    llog::info!(log_name, "service stop");
}
