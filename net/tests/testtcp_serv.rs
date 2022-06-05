use conf::conf::Conf;
use std::collections::HashMap;
use tokio::signal;
use tokio::sync::mpsc;
extern crate net;
use llog;

use net::{tcp::tcp_service, Communicate, ProtoMsgType, ProtoSender};

#[test]
fn testservice() {
    #[derive(Default)]
    struct TmpEntity {
        pub conn_map: HashMap<u64, ProtoSender>,
    }

    impl Communicate<ProtoMsgType> for TmpEntity {
        fn register(&mut self, vfd: u64, sender: ProtoSender) {
            self.conn_map.insert(vfd, sender);
        }
        fn unregister(&mut self, vfd: u64) {
            self.conn_map.remove(&vfd);
        }
        fn get(&mut self, vfd: u64) -> Option<&ProtoSender> {
            self.conn_map.get(&vfd)
        }
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = rt.block_on(async move {
        let entity = TmpEntity::default();
        let shared_state = net::ServiceState::new(entity, 10);

        // 这里的新连接产生对应的 channel 应该不需要考虑缓存队列,故只设置队列长度为1
        let (chan_out_tx, chan_out_rx) = mpsc::channel(1);
        let out_sender = shared_state.mailbox.send.clone();
        let (shutdown_complete_tx1, mut shutdown_complete_rx) = mpsc::channel::<()>(1);
        let shutdown_complete_tx2 = shutdown_complete_tx1.clone();
        let (shutdown_notify_tx, shutdown_notify_rx) = mpsc::channel::<()>(1);

        // tcp service
        tokio::spawn(async move {
            let log_name = "test_tcp_service.log";
            let conf = Conf::new();
            let addr = conf.get_tcp_serv_addr();
            tcp_service::start_service(
                net::ServiceType::Tcp,
                log_name,
                addr,
                signal::ctrl_c(),
                chan_out_tx,
                out_sender,
            )
            .await;
            drop(shutdown_complete_tx1);
            let _ = shutdown_notify_tx.send(());
        });

        // service
        tokio::spawn(async move {
            let log_name = "test_tcp_service_handler.log";
            let net::ServiceState { entity, mailbox } = shared_state;
            tcp_service::start_service_handler(
                log_name,
                chan_out_rx,
                entity,
                mailbox,
                shutdown_notify_rx,
            )
            .await;
            drop(shutdown_complete_tx2);
            llog::info!(log_name, "service stop");
        });

        let _ = shutdown_complete_rx.recv().await;
    });
}
