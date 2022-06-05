use net::{Communicate, ProtoMsgType, ProtoSender};
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct TcpSharedEntity {
    pub conn_map: HashMap<u64, ProtoSender>,
}

impl Communicate<ProtoMsgType> for TcpSharedEntity {
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

impl TcpSharedEntity {}
