use crate::game_modules::items::item_mgr::TitemMgr;
use crate::{
    errors::Error, game_modules::player::Tplayer, shared_states::GameSharedEntity, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait TgmItem {
    fn add_item(&mut self, value: Value) -> Result<()>;
}

impl TgmItem for GameSharedEntity {
    // args 需要三个参数: uid,item_id,stack
    fn add_item(&mut self, value: Value) -> Result<()> {
        #[derive(Serialize, Deserialize)]
        struct GmAddItemFormat {
            func: String,
            uid: u64,
            item_id: u32,
            stack: i32,
        }
        match serde_json::from_value::<GmAddItemFormat>(value) {
            Ok(gmobj) => self.reward_item_to_player(gmobj.uid, gmobj.item_id, gmobj.stack),
            Err(err) => {
                return Err(Error::from(err.to_string()));
            }
        }
    }
}
