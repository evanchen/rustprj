use conf::conf::Conf;
use tokio::signal;
extern crate net;
use crate::shared_states::{
    DbSharedEntity, GameSharedEntity, HttpSharedEntity, RpcSharedEntity, TcpSharedEntity,
};
use llog;
use net::{http::http_service, rpc::rpc_service, tcp::tcp_service, Communicate};
use tokio::{
    sync::mpsc,
    time::{self, Duration},
};

pub fn start() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = rt.block_on(async move {
        let sysconf = Conf::new();
        // :TODO: select! 宏没有办法按配置选择不同的 async 块,考虑把重复的代码提取出来
        if sysconf.get_host_type() == "game" {
            game_server_entry(sysconf).await;
        } else {
            db_server_entry(sysconf).await;
        }
    });
}

async fn game_server_entry(sysconf: Conf) {
    // 正常停止服务器,是要等所有 tcp/http/rpc等服务线程都停止后,进程退出
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<()>(1);
    // 这是一个关联性退出,比如 tcp 的服务退出了,其他也没必要再服务
    let (shutdown_notify_tx, mut shutdown_notify_rx) = mpsc::channel::<()>(1);
    // for player tcp connections
    let tcp_entity = TcpSharedEntity::default();
    let p_state = net::ServiceState::new(tcp_entity, 10);
    // 这里的 tcp 新连接会产生对应的对外传递消息的 channel, 应该不需要考虑缓存队列,故只设置队列长度为1
    let (p_chan_out_tx, mut p_chan_out_rx) = mpsc::channel(1);
    let p_out_sender = p_state.mailbox.send.clone();
    let p_shutdown_tx = shutdown_complete_tx.clone();
    let p_shutdown_notify_tx = shutdown_notify_tx.clone();

    // for http connections
    let http_entity = HttpSharedEntity::default();
    let (h_chan_out_tx, mut h_chan_out_rx) = mpsc::channel(1);
    let h_shutdown_tx = shutdown_complete_tx.clone();
    let h_shutdown_notify_tx = shutdown_notify_tx.clone();

    // for rpc connections
    let rpc_entity = RpcSharedEntity::new(sysconf.clone());
    let r_state = net::ServiceState::new(rpc_entity, 10);
    // 应该不需要考虑缓存队列,故只设置队列长度为1
    let (r_chan_out_tx, mut r_chan_out_rx) = mpsc::channel(1);
    let r_out_sender = r_state.mailbox.send.clone();
    let r_shutdown_tx = shutdown_complete_tx.clone();
    let r_shutdown_notify_tx = shutdown_notify_tx.clone();

    // rpc service, 本身就是一个 tpc service, 只不过监听服务端口不一样, 而且协议类型可能需要做区分
    let rpc_addr = sysconf.get_rpc_serv_addr().to_owned();
    tokio::spawn(async move {
        rpc_service::start_service(&rpc_addr, signal::ctrl_c(), r_chan_out_tx, r_out_sender).await;
        drop(r_shutdown_tx);
        let _ = r_shutdown_notify_tx.send(());
    });

    // http service
    let http_addr = sysconf.get_http_serv_addr().to_owned();
    tokio::spawn(async move {
        let addr = http_addr.parse().unwrap();
        http_service::start_service(addr, signal::ctrl_c(), h_chan_out_tx.clone()).await;
        drop(h_shutdown_tx);
        let _ = h_shutdown_notify_tx.send(()).await;
    });

    // player tcp service
    let tcp_addr = sysconf.get_tcp_serv_addr().to_owned();
    tokio::spawn(async move {
        let log_name = "palyer_tcp_service.log";
        tcp_service::start_service(
            net::ServiceType::Tcp,
            log_name,
            &tcp_addr,
            signal::ctrl_c(),
            p_chan_out_tx,
            p_out_sender,
        )
        .await;
        drop(p_shutdown_tx);
        let _ = p_shutdown_notify_tx.send(());
    });

    // tcp, http, rpc 等服务获得消息输出都会一个 loop 里进行处理,
    // 即,在一个大循环下处理数据状态共享(单线程处理游戏逻辑,但 io 非阻塞), 得到的输出再作为 tcp, http, rpc 的输入,返回结果.
    tokio::spawn(async move {
        let log_name = "game_service.log";
        let net::ServiceState { entity, mailbox } = p_state;
        let tcp_entity = entity;
        let mut p_mailbox = mailbox;

        let net::ServiceState { entity, mailbox } = r_state;
        let rpc_entity = entity;
        let mut r_mailbox = mailbox;

        // for game shared
        let mut game_entity = GameSharedEntity::new(sysconf, tcp_entity, rpc_entity, http_entity);

        let mut heart_beat = time::interval(Duration::from_millis(1000));
        loop {
            tokio::select! {
                // for player tcp service
                res = p_chan_out_rx.recv() => {
                    if let Some((vfd,sender)) = res {
                        game_entity.tcp_entity.register(vfd,sender);
                        llog::info!(log_name,"[tcp]: new client connection channel: vfd={}",vfd);
                    } else {
                        llog::error!(log_name,"p_chan_out_rx channel broken");
                        break;
                    }
                },
                res = p_mailbox.recv() => {
                    if let Some((vfd,proto_id,pto)) = res {
                        //println!("service get proto: vfd={},proto_id={}",vfd,proto_id);
                        let _ = game_entity.dispatch_tcp_msg(vfd,proto_id,pto).await;
                    } else {
                        llog::error!(log_name,"[tcp]: server service receive close");
                        break;
                    }
                }
                // for rpc service
                res = r_chan_out_rx.recv() => {
                    if let Some((vfd,sender)) = res {
                        game_entity.rpc_entity.register(vfd,sender);
                        llog::info!(log_name,"[rpc]: new client connection channel: vfd={}",vfd);
                    } else {
                        llog::error!(log_name,"r_chan_out_rx channel broken");
                        break;
                    }
                },
                res = r_mailbox.recv() => {
                    if let Some((vfd,proto_id,pto)) = res {
                        //println!("service get proto: vfd={},proto_id={}",vfd,proto_id);
                        let _ = game_entity.dispatch_rpc_msg(vfd,proto_id,pto).await;
                    } else {
                        llog::error!(log_name,"[rpc]: server service receive close");
                        break;
                    }
                }
                // for http service
                res = h_chan_out_rx.recv() => {
                    if let Some((hpt,optx)) = res {
                        let _ = game_entity.dispatch_http_msg(hpt,optx).await;
                    } else {
                        llog::error!(log_name, "[http]: service chan closed.");
                        break;
                    }
                },
                _ = heart_beat.tick() => {
                    println!("service heart_beat tick");
                }
                _ = shutdown_notify_rx.recv() => {
                    llog::error!(log_name,"server service shutdown");
                    break;
                }
            }
        }
        drop(shutdown_complete_tx);
        llog::info!(log_name, "all service stop");
    });
    let _ = shutdown_complete_rx.recv().await;
}

async fn db_server_entry(sysconf: Conf) {
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<()>(1);
    let (shutdown_notify_tx, mut shutdown_notify_rx) = mpsc::channel::<()>(1);

    // for rpc connections
    let rpc_entity = RpcSharedEntity::new(sysconf.clone());
    let r_state = net::ServiceState::new(rpc_entity, 10);
    // 应该不需要考虑缓存队列,故只设置队列长度为1
    let (r_chan_out_tx, mut r_chan_out_rx) = mpsc::channel(1);
    let r_out_sender = r_state.mailbox.send.clone();
    let r_shutdown_tx = shutdown_complete_tx.clone();
    let r_shutdown_notify_tx = shutdown_notify_tx.clone();

    // rpc service, 本身就是一个 tpc service, 只不过监听服务端口不一样, 而且协议类型可能需要做区分
    let rpc_db_addr = sysconf.get_rpc_db_serv_addr().to_owned();
    tokio::spawn(async move {
        rpc_service::start_service(&rpc_db_addr, signal::ctrl_c(), r_chan_out_tx, r_out_sender)
            .await;
        drop(r_shutdown_tx);
        let _ = r_shutdown_notify_tx.send(());
    });

    tokio::spawn(async move {
        let log_name = "db_service.log";

        let net::ServiceState { entity, mailbox } = r_state;
        let rpc_entity = entity;
        let mut r_mailbox = mailbox;

        // for db shared
        let mut db_entity = DbSharedEntity::new(sysconf, rpc_entity);

        let mut heart_beat = time::interval(Duration::from_millis(1000));
        loop {
            tokio::select! {
                // for rpc service
                res = r_chan_out_rx.recv() => {
                    if let Some((vfd,sender)) = res {
                        db_entity.rpc_entity.register(vfd,sender);
                        llog::info!(log_name,"[rpc]: new client connection channel: vfd={}",vfd);
                    } else {
                        llog::error!(log_name,"r_chan_out_rx channel broken");
                        break;
                    }
                },
                res = r_mailbox.recv() => {
                    if let Some((vfd,proto_id,pto)) = res {
                        //println!("service get proto: vfd={},proto_id={}",vfd,proto_id);
                        let _ = db_entity.dispatch_rpc_msg(vfd,proto_id,pto).await;
                    } else {
                        llog::error!(log_name,"[rpc]: server service receive close");
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
        drop(shutdown_complete_tx);
        llog::info!(log_name, "all service stop");
    });
    let _ = shutdown_complete_rx.recv().await;
}
