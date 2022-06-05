use conf::conf::Conf;
use tokio::signal;
use tokio::sync::mpsc;
extern crate net;
use llog;
use net::http::http_service;
use net::http::{ChanHttpProtoReceiverOp, HttpProtoType};
use tokio::time::{self, Duration};

#[test]
fn test_http_service() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = rt.block_on(async move {
        let (chan_out_tx, chan_out_rx) = mpsc::channel(1);
        let (shutdown_complete_tx1, mut shutdown_complete_rx) = mpsc::channel::<()>(1);
        let shutdown_complete_tx2 = shutdown_complete_tx1.clone();
        let (shutdown_notify_tx, shutdown_notify_rx) = mpsc::channel::<()>(1);

        // http service
        tokio::spawn(async move {
            let conf = Conf::new();
            let addr = conf.get_http_serv_addr();
            let addr = addr.parse().unwrap();
            http_service::start_service(addr, signal::ctrl_c(), chan_out_tx.clone()).await;
            drop(shutdown_complete_tx1);
            let _ = shutdown_notify_tx.send(()).await;
        });

        // service
        tokio::spawn(async move {
            let logger = "test_http_serv.log";
            start_service_handler(chan_out_rx, shutdown_notify_rx).await;
            drop(shutdown_complete_tx2);
            llog::info!(logger, "service stop");
        });

        let _ = shutdown_complete_rx.recv().await;
    });
}

pub async fn start_service_handler(
    mut chan_out_rx: ChanHttpProtoReceiverOp,
    mut shutdown_notify_rx: mpsc::Receiver<()>,
) {
    let logger = "http_handler.log";
    let mut interval = time::interval(Duration::from_millis(1000));
    loop {
        tokio::select! {
            res = chan_out_rx.recv() => {
                match res {
                    Some((hpt,optx)) => {
                        let res = match hpt {
                            HttpProtoType::ReqServerInfo(hostid) => {
                                let hostinfo = format!("name: s1, host: s1.xxx.com:8081, hostid: {}", hostid);
                                HttpProtoType::RespServerInfo(hostinfo)
                            },
                            _ => {
                                let hostinfo = format!("unimplemented,{:?}",hpt);
                                HttpProtoType::Unimplemented(hostinfo)
                            },
                        };
                        let _ = optx.send(res);
                    },
                    None => {
                        llog::error!(logger, "service chan closed.");
                        break;
                    }
                }
            },
            _ = interval.tick() => {
                println!("http service timer tick");
            }
            _ = shutdown_notify_rx.recv() => {
                break;
            }
        }
    }
}
