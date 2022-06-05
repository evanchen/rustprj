use super::GameSharedEntity;
use crate::game_modules::gm::Tgm;
use net::http::{HttpProtoSenderOp, HttpProtoType};

#[derive(Debug, Default)]
pub struct HttpSharedEntity {}

pub fn dispatch_http_msg(
    game_entity: &mut GameSharedEntity,
    hpt: HttpProtoType,
    optx: HttpProtoSenderOp,
) {
    let res = match hpt {
        HttpProtoType::ReqServerInfo(hostid) => {
            let res = format!("name: s1, host: s1.xxx.com:8081, hostid: {}", hostid);
            HttpProtoType::RespServerInfo(res)
        }
        HttpProtoType::ReqGM(cmdstr) => match game_entity.handler_gm_cmd(cmdstr) {
            Ok(_res) => HttpProtoType::RespGM("gm executed".to_string()),
            Err(err) => HttpProtoType::RespGM(err.to_string()),
        },
        _ => {
            let res = format!("unimplemented,{:?}", hpt);
            HttpProtoType::Unimplemented(res)
        }
    };
    println!("http dispatch msg");
    let _ = optx.send(res);
}
