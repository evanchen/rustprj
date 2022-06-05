use crate::errors::Error;
use crate::game_modules::player::{Player, Tplayer};
use crate::shared_states::GameSharedEntity;
use crate::Result;
use net::utils;
use net::ProtoType;
use proto::ptoout::*;

const LOG_NAME: &str = "player_handler.log";

pub fn s_player_brief(game_entity: &mut GameSharedEntity, vfd: u64, _pto: ProtoType) -> Result<()> {
    let player = game_entity.get_player_by_vfd(vfd);
    if player.is_none() {
        return Ok(());
    }
    let player = player.unwrap();
    let sendptoid = c_player_brief::c_player_brief::id();
    let sendpto = c_player_brief::c_player_brief {
        acc: player.get_acc().to_string(),
        uid: player.get_uid(),
        name: player.get_name().to_string(),
        level: player.get_level(),
        expr: player.get_expr(),
        money: player.get_money(),
    };
    let sendpto = ProtoType::c_player_brief(sendpto);
    player.send(sendptoid, sendpto);
    Ok(())
}
