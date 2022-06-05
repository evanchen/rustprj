use super::client_send_only;
use crate::{utils, Communicate, ProtoMsgType, ProtoSender, ProtoType};
use conf::conf::Conf;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct RpcSender {
    conf: Conf,
    pub chan_map: HashMap<u64, ProtoSender>,
}

impl Communicate<ProtoMsgType> for RpcSender {
    fn register(&mut self, vfd: u64, sender: ProtoSender) {
        self.chan_map.insert(vfd, sender);
    }
    fn unregister(&mut self, vfd: u64) {
        self.chan_map.remove(&vfd);
    }
    fn get(&mut self, vfd: u64) -> Option<&ProtoSender> {
        self.chan_map.get(&vfd)
    }
}

impl RpcSender {
    pub fn new(conf: Conf) -> RpcSender {
        RpcSender {
            conf,
            chan_map: HashMap::new(),
        }
    }

    fn new_connection(&mut self, host_id: u64, addr: &str) -> std::io::Result<()> {
        println!(
            "start a new rpc connection,host_id={},addr={}",
            host_id, addr
        );

        // 这个 rpc 的连接不允许断开主动连接.如果连接因为其他问题断开,则上面的 send 会出现失败
        let std_stream = std::net::TcpStream::connect(addr)?;
        std_stream.set_nonblocking(true)?;
        let stream = tokio::net::TcpStream::from_std(std_stream)?;

        let (tx, rx) = mpsc::channel(1000);
        self.chan_map.insert(host_id, tx);
        tokio::spawn(async move {
            let log_name = format!("client_send_only_host_id_{}.log", host_id);
            let _ = client_send_only::start_service(stream, &log_name, host_id, rx).await;
        });
        Ok(())
    }

    pub fn send2host(&mut self, host_id: u64, proto_id: u32, pto: ProtoType) {
        let cur_host_id = self.conf.get_host_id();
        if host_id == cur_host_id {
            return;
        }
        let pto = if let Some(tx) = self.chan_map.get(&host_id) {
            let pto = match utils::try_send("rpc_sender.log", tx, host_id, proto_id, pto) {
                None => {
                    return;
                }
                Some((1, _)) => {
                    println!(
                        "[send2host]: chan_full,host_id={},proto_id:{}",
                        host_id, proto_id
                    );
                    return;
                }
                Some((_, pto)) => pto, // need new connection
            };
            pto
        } else {
            pto
        };

        let addr = if host_id == self.conf.get_db_host_id() {
            self.conf.get_rpc_db_serv_addr().to_owned()
        } else {
            // :TODO: get addr by host_id
            let addr = format!("127.0.0.1:8083");
            addr
        };
        match self.new_connection(host_id, &addr) {
            Ok(_) => {
                let tx = self.chan_map.get(&host_id).unwrap();
                utils::try_send("rpc_sender.log", tx, host_id, proto_id, pto);
            }
            Err(err) => {
                println!(
                    "[send2host]: host_id={},{},connection failed: {}",
                    host_id, addr, err
                );
            }
        }
    }

    pub fn send2db(&mut self, proto_id: u32, pto: ProtoType) {
        self.send2host(self.conf.get_db_host_id(), proto_id, pto);
    }
}
