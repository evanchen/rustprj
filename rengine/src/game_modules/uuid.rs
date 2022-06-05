use super::db::{load, DBConf, DBObj};
use crate::shared_states::{GameSharedEntity, RpcSharedEntity};
use core::panic;
use net::ProtoType;
use serde::{Deserialize, Serialize};

const LOG_NAME: &str = "uuid.log";
// 假设服务器id支持从 1 到 9999
const BASE: u64 = 10000;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct UUID {
    counter: u64,
    host_id: u64,
    for_player: u64,
    for_item: u64,
    #[serde(skip)]
    inner: Option<DBObj>,
}

impl UUID {
    pub fn new(host_id: u64, base_player_uid: u64, base_item_uid: u64) -> Self {
        if host_id >= BASE {
            panic!("max host_id: {} >= {}", host_id, BASE);
        }
        UUID {
            host_id,
            for_player: base_player_uid,
            for_item: base_item_uid,
            inner: None,
            ..Default::default()
        }
    }

    pub fn build_with_db(mut self, dbobj: DBObj) -> Self {
        self.inner = Some(dbobj);
        self
    }

    pub fn inc_player_uid(&mut self) -> u64 {
        self.for_player += 1;
        let puid = self.for_player * BASE + self.host_id;
        puid
    }

    pub fn inc_item_uid(&mut self) -> u64 {
        self.for_item += 1;
        let iuid = self.for_item * BASE + self.host_id;
        iuid
    }

    pub fn load_ret(game_entity: &mut GameSharedEntity, pto: ProtoType) {
        if game_entity.uuid.is_some() {
            println!("[uuid.load_ret]: is all setted!");
            return;
        }

        let ptoobj = match pto {
            ProtoType::db_load_resp(ptoobj) => ptoobj,
            _ => return,
        };

        let host_id = game_entity.get_host_id();
        if ptoobj.value.len() == 0 {
            llog::info!(LOG_NAME, "[uuid.load_ret]: new uuid");
            let dbobj = DBObj::new(host_id, DBConf::UUID);
            let mut uuid = UUID::new(host_id, 0, 0).build_with_db(dbobj);
            uuid.save(&mut game_entity.rpc_entity);
            game_entity.uuid = Some(uuid);
            return;
        }

        let uuid: UUID = match serde_json::from_slice(&ptoobj.value) {
            Ok(uuid) => uuid,
            Err(err) => {
                llog::error!(LOG_NAME, "[uuid.load_ret]: err={:?}", err);
                return;
            }
        };
        let dbobj = DBObj::new(host_id, DBConf::UUID);
        game_entity.uuid = Some(uuid.build_with_db(dbobj));
    }

    pub fn save(&mut self, rpc_entity: &mut RpcSharedEntity) {
        self.counter += 1;
        let datastr = serde_json::to_vec(self).unwrap(); // shouldn't failed!
        self.inner
            .as_ref()
            .unwrap()
            .save(rpc_entity, self.counter, datastr)
    }
}

pub trait Tuuid {
    fn uuid_init(&mut self, host_id: u64);
    fn new_player_uid(&mut self) -> u64;
    fn new_item_uid(&mut self) -> u64;
}

impl Tuuid for GameSharedEntity {
    fn uuid_init(&mut self, host_id: u64) {
        load(&mut self.rpc_entity, host_id, DBConf::UUID);
    }

    fn new_player_uid(&mut self) -> u64 {
        let uuid = self.uuid.as_mut().unwrap();
        let uid = uuid.inc_player_uid();
        uuid.save(&mut self.rpc_entity);
        uid
    }

    fn new_item_uid(&mut self) -> u64 {
        let uuid = self.uuid.as_mut().unwrap();
        let uid = uuid.inc_item_uid();
        uuid.save(&mut self.rpc_entity);
        uid
    }
}
