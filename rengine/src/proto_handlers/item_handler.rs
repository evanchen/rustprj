use crate::errors::Error;
use crate::game_modules::bag::BagType;
use crate::game_modules::player::{Player, Tplayer};
use crate::shared_states::GameSharedEntity;
use crate::Result;
use net::{allptos, utils, ProtoType};
use proto::ptoout::*;
use proto::{c_item_bag::c_item_bag, item_info::item_info};

const LOG_NAME: &str = "item_handler.log";

pub fn s_item_bag(game_entity: &mut GameSharedEntity, vfd: u64, pto: ProtoType) -> Result<()> {
    let player = game_entity.get_player_by_vfd(vfd);
    if player.is_none() {
        return Ok(());
    }
    let player = player.unwrap();
    let ptoobj = match pto {
        ProtoType::s_item_bag(ptoobj) => ptoobj,
        _ => return Ok(()),
    };

    let bag_type = BagType::from_u8(ptoobj.bagtype);
    let item_mgr = player.get_item_mgr();
    let pack_info = item_mgr.pack_bag_info(bag_type);
    let sendptoid = c_item_bag::id();
    let sendpto = c_item_bag {
        bagtype: ptoobj.bagtype,
        uid: player.get_uid(),
        baginfo: pack_info,
    };
    let sendpto = ProtoType::c_item_bag(sendpto);
    player.send(sendptoid, sendpto);
    Ok(())
}
