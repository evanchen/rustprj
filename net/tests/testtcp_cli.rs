use conf::conf::Conf;
use tokio::signal;
use tokio::{
    sync::mpsc,
    time::{self, Duration},
};
extern crate net;
use llog;

use net::{tcp::client, Communicate, ProtoMsgType, ProtoSender};
use proto::allptos::ProtoType;

#[test]
fn testclient() {
    struct TmpEntity {
        pub service_sender: Option<ProtoSender>,
    }

    impl net::Communicate<ProtoMsgType> for TmpEntity {
        fn register(&mut self, _vfd: u64, sender: ProtoSender) {
            assert!(self.service_sender.is_none());
            self.service_sender = Some(sender);
        }
        fn unregister(&mut self, _vfd: u64) {
            self.service_sender = None;
        }
        fn get(&mut self, _vfd: u64) -> Option<&ProtoSender> {
            self.service_sender.as_ref()
        }
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = rt.block_on(async move {
        let entity = TmpEntity {
            service_sender: None,
        };
        let shared_state = net::ServiceState::new(entity, 10);
        let (chan_out_tx,mut chan_out_rx) = mpsc::channel(1);
        let out_sender = shared_state.mailbox.send.clone();

        let (shutdown_complete_tx1, mut shutdown_complete_rx) = mpsc::channel::<()>(1);
        let shutdown_complete_tx2 = shutdown_complete_tx1.clone();
        let (shutdown_notify_tx, mut shutdown_notify_rx) = mpsc::channel::<()>(1);

        // tcp service
        let identity = 1u64;
        tokio::spawn(async move {
            let conf = Conf::new();
            let addr = conf.get_tcp_serv_addr().to_owned();
            let _ = client::run(addr, signal::ctrl_c(),identity,chan_out_tx.clone(),out_sender).await;
            drop(shutdown_complete_tx1);
            let _ = shutdown_notify_tx.send(());
        });

        // service
        tokio::spawn(async move {
            let log_name = "test_service_cli.log";
            let net::ServiceState {
                mut entity,
                mut mailbox,
            } = shared_state;

            let stopsend = false;
            let mut interval = time::interval(Duration::from_millis(50));
            loop {
                tokio::select! {
                    res = chan_out_rx.recv() => {
                        if let Some((vfd,sender)) = res {
                            entity.register(vfd,sender);
                            println!("new client connection channel");
                        } else {
                            println!("chan_out_rx channel broken");
                            break;
                        }
                    },
                    res = mailbox.recv() => {
                        if let Some((vfd,proto_id,pto)) = res {
                            //println!("service get proto: vfd={},proto_id={}",vfd,proto_id);
                            // 处理协议,并返回结果(协议).这里测试我们直接返回接收到的协议
                            match entity.get(vfd) {
                                Some(ch) => {
                                    if let Err(err) = ch.send((vfd,proto_id,pto)).await {
                                        llog::error!(log_name,"service send proto to connection failed: vfd={},proto_id={},err={:?}",vfd,proto_id,err);
                                    }
                                },
                                None => {
                                    // ch 不存在
                                    llog::error!(log_name,"service send proto,connection doesn't exist: vfd={},proto_id={}",vfd,proto_id);
                                }
                            }
                        }
                    }
                    _ = interval.tick() => {
                        // 客户端每一 tick 发送一个协议
                        let s_login = proto::s_login::s_login::default_with_random_value();
                        let vfd = identity;
                        let proto_id = 101;
                        if !stopsend {
                            match entity.get(vfd) {
                                Some(ch) => {
                                    if let Err(err) = ch.send((vfd,proto_id,ProtoType::s_login(s_login))).await {
                                        llog::error!(log_name,"tick send proto to connection failed: vfd={},proto_id={},err={:?}",vfd,proto_id,err);
                                        entity.unregister(vfd);
                                    }
                                    //stopsend = true;
                                },
                                None => {
                                    // ch 不存在
                                    llog::error!(log_name,"tick send proto,connection doesn't exist: vfd={},proto_id={}",vfd,proto_id);
                                }
                            }
                            //println!("tick send vfd={},proto_id={}",vfd,proto_id);
                            println!("service timer tick");
                        }
                    }
                    _ = shutdown_notify_rx.recv() => {
                        break;
                    }
                }
            }
            drop(shutdown_complete_tx2);
            llog::info!(log_name,"service stop");
        });

        let _ = shutdown_complete_rx.recv().await;
    });
}
