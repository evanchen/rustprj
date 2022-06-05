use crate::shared_states::GameSharedEntity;
use crate::{
    game_modules::{
        bag::{Bag, BagType},
        items::Item,
        player::{self, Player, Tplayer},
        uuid::Tuuid,
    },
    shared_states::game_state,
    Result,
};
use proto::{c_item_bag::c_item_bag, item_info::item_info};
use serde::{Deserialize, Serialize};

const LOG_NAME: &str = "item_mgr.log";

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ItemMgr {
    owner: u64,
    pub bag_equiped: Bag,
    pub bag_items: Bag,
    #[serde(skip)]
    pub bag_temp: Bag,
}

impl ItemMgr {
    pub fn new(owner: u64) -> Self {
        let bag_equiped = Bag::new(BagType::Equiped, 5);
        let bag_items = Bag::new(BagType::Items, 1000);
        let bag_temp = Bag::new(BagType::Temp, 500);

        ItemMgr {
            owner,
            bag_equiped,
            bag_items,
            bag_temp,
        }
    }

    pub fn add_item_to_bag(&mut self, bag_type: BagType, new_item: Item) -> Result<()> {
        let bag = match bag_type {
            BagType::Equiped => &mut self.bag_equiped,
            BagType::Items => &mut self.bag_items,
            BagType::Temp => &mut self.bag_temp,
        };
        let item_uid = new_item.uid();
        match bag.add_item(new_item) {
            Err(err) => {
                llog::info!(
                    LOG_NAME,
                    "[add_item_to_bag]: owner={},bag_type={},{}",
                    self.owner,
                    bag_type.into_u8(),
                    err
                );
                return Err(err);
            }
            _ => {}
        }
        let item = bag.get_item(item_uid).unwrap();
        llog::info!(
            LOG_NAME,
            "[add_item_to_bag]: owner={},bag_type={},{}",
            self.owner,
            bag_type.into_u8(),
            item.log_info()
        );
        Ok(())
    }

    pub fn get_bag_item(&mut self, bag_type: BagType, item_uid: u64) -> Option<&mut Item> {
        let bag = match bag_type {
            BagType::Equiped => &mut self.bag_equiped,
            BagType::Items => &mut self.bag_items,
            BagType::Temp => &mut self.bag_temp,
        };
        bag.get_item_mut(item_uid)
    }

    pub fn pack_bag_info(&self, bag_type: BagType) -> Vec<item_info> {
        match bag_type {
            BagType::Equiped => self.bag_equiped.pack(),
            BagType::Items => self.bag_equiped.pack(),
            BagType::Temp => self.bag_temp.pack(),
            _ => vec![],
        }
    }
}

pub fn create_new_item(item_uid: u64, id: u32, stack: i32) -> Item {
    let new_item = Item::new(item_uid, id, stack);
    new_item
}

pub trait TitemMgr {
    fn reward_item_to_player(&mut self, uid: u64, item_id: u32, stack: i32) -> Result<()>;
}

impl TitemMgr for GameSharedEntity {
    fn reward_item_to_player(&mut self, uid: u64, item_id: u32, stack: i32) -> Result<()> {
        player::player_do_mut(self, uid, |game_state, player| {
            let item_mgr = player.get_item_mgr();
            let item_uid = game_state.new_item_uid();
            let new_item = create_new_item(item_uid, item_id, stack);
            item_mgr.add_item_to_bag(BagType::Items, new_item)
        })
    }
}
