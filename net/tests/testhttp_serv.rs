use conf::conf::Conf;
use tokio::signal;
use tokio::sync::mpsc;
extern crate net;
use llog;

use net::http::http_service;

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
            let port = conf.get_http_port();
            // Bind a TCP listener
            let addr = format!("127.0.0.1:{}", port);
            let addr = addr.parse().unwrap();
            http_service::start_service(addr, signal::ctrl_c(), chan_out_tx.clone()).await;
            drop(shutdown_complete_tx1);
            let _ = shutdown_notify_tx.send(()).await;
        });

        // service
        tokio::spawn(async move {
            let logger = "test_http_serv.log";
            http_service::start_service_handler(chan_out_rx, shutdown_notify_rx).await;
            drop(shutdown_complete_tx2);
            llog::info!(logger, "service stop");
        });

        let _ = shutdown_complete_rx.recv().await;
    });
}
