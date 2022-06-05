use crate::game_modules::db::{load, DBConf};
use crate::game_modules::player::{Player, Tplayer};
use crate::shared_states::GameSharedEntity;
use crate::Result;
use net::Communicate;
use net::{allptos, utils, ProtoType};
use proto::ptoout::*;

const LOG_NAME: &str = "login_handler.log";

pub fn s_login(game_entity: &mut GameSharedEntity, vfd: u64, pto: ProtoType) -> Result<()> {
    let ch = match game_entity.tcp_entity.get(vfd) {
        Some(ch) => ch,
        None => return Ok(()),
    };
    let ptoobj = match pto {
        ProtoType::s_login(ptoobj) => ptoobj,
        _ => return Ok(()),
    };

    let sendptoid = c_login::c_login::id();
    let mut c_login = c_login::c_login::default();
    c_login.ret = 1;
    c_login.magic = 0;

    if !allptos::is_proto_version(&ptoobj.vers) {
        let sendpto = ProtoType::c_login(c_login);
        utils::try_send(LOG_NAME, ch, vfd, sendptoid, sendpto);
        return Ok(());
    }

    // 判断本次操作是否正在进行中
    let op_ukey = format!("{}", vfd);
    if !game_entity.op_entity.can_start_op(op_ukey) {
        utils::feekback(LOG_NAME, ch, vfd, 111, String::from(""));
        return Ok(());
    }

    game_entity.add_uid_acc(vfd, 0, ptoobj.acc.clone());

    // 加载数据,等待数据返回后,再进行下一步
    let host_id = game_entity.get_host_id();
    load(
        &mut game_entity.rpc_entity,
        host_id,
        DBConf::Player(ptoobj.acc),
    );
    Ok(())
}
