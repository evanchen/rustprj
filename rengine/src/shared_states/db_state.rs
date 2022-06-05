use super::RpcSharedEntity;
use crate::{proto_handlers::RpcDbSendFuncMarker, Result};
use conf::conf::Conf;
use net::ProtoType;
use std::collections::HashMap;

const LOG_NAME: &str = "db_state.log";

pub struct DbSharedEntity {
    sysconf: Conf,
    pub rpc_entity: RpcSharedEntity,
    datas: HashMap<String, (u64, Vec<u8>)>, // <addr,(counter,buf)>
}

impl DbSharedEntity {
    pub fn new(sysconf: Conf, rpc_entity: RpcSharedEntity) -> Self {
        let db_entity = DbSharedEntity {
            sysconf,
            rpc_entity,
            datas: HashMap::new(),
        };
        db_entity
    }

    // :TODO: 以 db 接口代替
    pub fn get(&mut self, key: &str) -> Option<&(u64, Vec<u8>)> {
        println!("[db_state.get]: key={}", key);
        self.datas.get(key)
    }

    pub fn set(&mut self, key: String, value: (u64, Vec<u8>)) {
        println!("[db_state.set]: key={},value={:?}", key, value);
        self.datas.insert(key, value);
    }

    pub fn del(&mut self, key: &str) {
        println!("[db_state.del]: key={}", key);
        self.datas.remove(key);
    }

    pub async fn dispatch_rpc_msg(
        &mut self,
        vfd: u64,
        proto_id: u32,
        pto: ProtoType,
    ) -> Result<()> {
        let (pid, proto_name) = pto.inner_info();
        let proto_func = RpcDbSendFuncMarker::from_str(proto_name).into_func();
        if proto_func.is_none() {
            llog::info!(
                LOG_NAME,
                "[tcp.dispatch_rpc_msg]: protocol id does not match: {},{},{}",
                proto_id,
                pid,
                proto_name
            );
            return Ok(());
        }
        let proto_func = proto_func.unwrap();
        if let Err(err) = proto_func(self, vfd, pto) {
            llog::info!(LOG_NAME, "[tcp.dispatch_rpc_msg]: {}", err);
        }
        Ok(())
    }
}
