use crate::shared_states::GameSharedEntity;
use crate::Result;
use serde_json::Value;
pub type DBRetFunc = fn(game_entity: &mut GameSharedEntity, value: Value) -> Result<()>;

pub mod gm_item;
pub use gm_item::TgmItem;

pub mod gm_trait;
pub use gm_trait::Tgm;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum GMFuncMarker {
    add_item,
    unknow,
}

impl GMFuncMarker {
    pub fn into_func(self) -> Option<DBRetFunc> {
        match self {
            GMFuncMarker::add_item => Some(GameSharedEntity::add_item),
            _ => None,
        }
    }

    pub fn from_str(func_name: &str) -> Self {
        match func_name {
            "add_item" => GMFuncMarker::add_item,
            _ => GMFuncMarker::unknow,
        }
    }
}
