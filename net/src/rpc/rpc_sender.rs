use super::client_send_only;
use crate::{Communicate, ProtoMsgType, ProtoSender, ProtoType};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Default)]
pub struct RpcSender {
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
    pub fn new() -> RpcSender {
        RpcSender {
            chan_map: HashMap::new(),
        }
    }

    pub async fn send2host(
        &mut self,
        hostid: u64,
        proto_id: u32,
        pto: ProtoType,
    ) -> crate::Result<()> {
        if let Some(tx) = self.chan_map.get_mut(&hostid) {
            tx.send((hostid, proto_id, pto)).await?;
        } else {
            println!("start a new rpc connection,hostid={}", hostid);
            // Establish a connection
            // :TODO: 根据 hostid 获取指定的对端 rpc 的ip地址
            let addr = format!("127.0.0.1:8081");
            let (tx, rx) = mpsc::channel(1000);
            // 因为这里是异步启动的连接,所以在未初始化与对端的 tcp 连接而连续发送消息, tx 可能会被后来的 insert 替换...
            // :TODO: 加个 ch 来通知,或加个 sleep ? 但如果对端无连接监听的话,会一直影响当前 task, 考虑在启动服务器时做一次 send 操作.
            self.chan_map.insert(hostid, tx);
            // 这个 rpc 的连接不允许断开主动连接.如果连接因为其他问题断开,则上面的 send 会出现失败
            tokio::spawn(async move {
                let _ =
                    client_send_only::start_service(addr, "client_send_only.log", hostid, rx).await;
            });
        }
        Ok(())
    }
}
