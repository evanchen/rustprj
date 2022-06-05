use conf::conf::Conf;
use net::{
    rpc::rpc_sender::{self, RpcSender},
    Communicate, ProtoMsgType, ProtoSender, ProtoType,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct RpcSharedEntity {
    pub conn_map: HashMap<u64, ProtoSender>,
    inner: rpc_sender::RpcSender,
}

impl Communicate<ProtoMsgType> for RpcSharedEntity {
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

impl RpcSharedEntity {
    pub fn new(conf: Conf) -> Self {
        RpcSharedEntity {
            conn_map: HashMap::new(),
            inner: RpcSender::new(conf),
        }
    }

    pub fn send2host(&mut self, hostid: u64, proto_id: u32, pto: ProtoType) {
        self.inner.send2host(hostid, proto_id, pto);
    }

    pub fn send2db(&mut self, proto_id: u32, pto: ProtoType) {
        self.inner.send2db(proto_id, pto);
    }
}
