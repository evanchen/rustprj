use super::{ChanHttpProtoReceiverOp, ChanHttpProtoSenderOp, HttpProtoType};
use std::future::Future;
use std::net::SocketAddr;

use std::convert::Infallible;
use tokio::{
    sync::mpsc,
    sync::oneshot,
    time::{self, Duration},
};
use warp::Filter;
extern crate llog;

const LOG_NAME: &str = "http.log";

fn with_sender(
    sender: ChanHttpProtoSenderOp,
) -> impl Filter<Extract = (ChanHttpProtoSenderOp,), Error = Infallible> + Clone {
    warp::any().map(move || sender.clone())
}

pub async fn start_service(
    addr: SocketAddr,
    shutdown: impl Future,
    chan_out: ChanHttpProtoSenderOp,
) {
    // get /req/server/all
    let handler_req_server_all = warp::get()
        .and(warp::path!("req" / "server" / "all"))
        .map(|| "name: s1, host: s1.xxx.com:8081");

    let chan_out_req_server = chan_out.clone();
    // get /req/server/:u32
    // builds on example2 but adds custom error handling
    let handler_req_server = warp::get()
        .and(warp::path!("req" / "server" / u32))
        .and(with_sender(chan_out_req_server))
        .then(req_server)
        .map(|res| res);

    let routes = handler_req_server_all.or(handler_req_server);
    tokio::select! {
        _ = warp::serve(routes).run(addr) => {
            llog::error!(LOG_NAME,"http.run closed.");
        }
        _ = shutdown => {
            llog::info!(LOG_NAME,"http.run shut down.");
        }
    }
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

async fn req_server(hostid: u32, chan_out: ChanHttpProtoSenderOp) -> String {
    let (optx, oprx) = oneshot::channel();
    tokio::spawn(async move {
        //这里不需要 try_send, 如果 channel 阻塞,就让它一直阻塞, 因为send出去的是 oneshot 的 chan
        if let Err(err) = chan_out
            .send((HttpProtoType::ReqServerInfo(hostid), optx))
            .await
        {
            llog::error!(
                LOG_NAME,
                "req_server, chan full: hostid={},err={:?}",
                hostid,
                err
            );
        }
    });
    match oprx.await {
        Ok(hpt) => {
            format!("success.\n{:?}", hpt)
        }
        Err(err) => {
            llog::error!(LOG_NAME, "/req/server/{},err={:?}", hostid, err);
            format!("failed,{}", err)
        }
    }
}
