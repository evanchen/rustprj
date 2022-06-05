pub mod dbobj;
use crate::game_modules::{player::Player, uuid::UUID};
use crate::shared_states::GameSharedEntity;
pub use dbobj::load;
pub use dbobj::DBObj;
use net::ProtoType;

pub type DBRetFunc = fn(game_entity: &mut GameSharedEntity, pto: ProtoType);

#[derive(Debug)]
pub enum DBConf {
    UUID,
    Player(String),
}

// db 数据与处理函数的映射
#[derive(Debug)]
pub enum DBRetFuncMarker {
    UUIDLoadFunc = 1,
    PlayerLoadFunc = 2,
    Unknow = 99999999,
}

impl DBConf {
    // 返回 dbname,key,ret_func
    pub fn info(&self) -> (String, String, DBRetFuncMarker) {
        match self {
            DBConf::UUID => (
                String::from("uuiddb"),
                String::from("uuid.dat"),
                DBRetFuncMarker::UUIDLoadFunc,
            ),
            DBConf::Player(acc) => (
                String::from("playerdb"),
                format!("{}.dat", acc),
                DBRetFuncMarker::PlayerLoadFunc,
            ),
        }
    }
}

impl DBRetFuncMarker {
    pub fn into_func(self) -> Option<DBRetFunc> {
        match self {
            DBRetFuncMarker::UUIDLoadFunc => Some(UUID::load_ret),
            DBRetFuncMarker::PlayerLoadFunc => Some(Player::load_ret),
            _ => None,
        }
    }

    pub fn into_u64(self) -> u64 {
        self as u64
    }

    pub fn from_u64(tag: u64) -> Self {
        match tag {
            1 => DBRetFuncMarker::UUIDLoadFunc,
            2 => DBRetFuncMarker::PlayerLoadFunc,
            _ => DBRetFuncMarker::Unknow,
        }
    }
}
