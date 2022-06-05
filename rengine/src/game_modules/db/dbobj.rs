use crate::game_modules::db::DBConf;
use crate::shared_states::RpcSharedEntity;
use net::ProtoType;
use proto::db_load_req::db_load_req;
use proto::db_save_req::db_save_req;

#[derive(Debug)]
pub struct DBObj {
    host_id: u64,
    conf: DBConf,
}

impl DBObj {
    pub fn new(host_id: u64, conf: DBConf) -> Self {
        DBObj { host_id, conf }
    }

    pub fn load(&self, rpc_entity: &mut RpcSharedEntity) {
        let (dbname, key, ret_func) = self.conf.info();
        let proto_id = db_load_req::id();
        let db_load_req = db_load_req {
            from_host: self.host_id,
            db_name: dbname,
            key,
            ret_func: ret_func.into_u64(),
            vfd: 0,
        };
        let pto = ProtoType::db_load_req(db_load_req);
        rpc_entity.send2db(proto_id, pto)
    }

    pub fn save(&self, rpc_entity: &mut RpcSharedEntity, counter: u64, datastr: Vec<u8>) {
        let (dbname, key, _) = self.conf.info();
        let proto_id = db_save_req::id();
        let db_save_req = db_save_req {
            from_host: self.host_id,
            db_name: dbname,
            key,
            value: datastr,
            counter,
        };
        let pto = ProtoType::db_save_req(db_save_req);
        rpc_entity.send2db(proto_id, pto)
    }
}

pub fn load(rpc_entity: &mut RpcSharedEntity, host_id: u64, conf: DBConf) {
    let tmp = DBObj::new(host_id, conf);
    tmp.load(rpc_entity);
}
