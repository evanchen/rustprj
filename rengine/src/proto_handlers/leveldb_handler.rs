use crate::{
    errors::Error,
    game_modules::{
        db::DBRetFuncMarker,
        uuid::{self, UUID},
    },
    shared_states::{DbSharedEntity, GameSharedEntity},
    Result,
};
use llog;
use net::ProtoType;
use proto::ptoout::*;

const LOG_NAME: &str = "leveldb_handler.log";

pub fn db_load_req(db_entity: &mut DbSharedEntity, _vfd: u64, pto: ProtoType) -> Result<()> {
    let ptoobj = match pto {
        ProtoType::db_load_req(ptoobj) => ptoobj,
        _ => return Err(Error::Feedback((111, String::from("")))),
    };

    let sendptoid = db_load_resp::db_load_resp::id();

    let from_host = ptoobj.from_host;
    let ukey = format!("{}/{}", ptoobj.db_name, ptoobj.key);
    let resp = match db_entity.get(&ukey) {
        Some((counter, value)) => db_load_resp::db_load_resp {
            key: ptoobj.key,
            value: value.clone(),
            counter: *counter,
            ret_func: ptoobj.ret_func,
            vfd: ptoobj.vfd,
        },
        None => db_load_resp::db_load_resp {
            key: ptoobj.key,
            value: Vec::new(),
            counter: 0,
            ret_func: ptoobj.ret_func,
            vfd: ptoobj.vfd,
        },
    };

    let sendpto = ProtoType::db_load_resp(resp);
    db_entity
        .rpc_entity
        .send2host(from_host, sendptoid, sendpto);
    Ok(())
}

pub fn db_save_req(db_entity: &mut DbSharedEntity, _vfd: u64, pto: ProtoType) -> Result<()> {
    let ptoobj = match pto {
        ProtoType::db_save_req(ptoobj) => ptoobj,
        _ => return Ok(()),
    };

    let ukey = format!("{}/{}", ptoobj.db_name, ptoobj.key);
    if let Some((counter, _)) = db_entity.get(&ukey) {
        if ptoobj.counter <= *counter {
            llog::info!(
                LOG_NAME,
                "[db_save_req]: save counter failed: {},{},<=,{}",
                ukey,
                ptoobj.counter,
                *counter
            );
            return Ok(());
        }
    }
    let tvalue = (ptoobj.counter, ptoobj.value);
    db_entity.set(ukey, tvalue);
    Ok(())
}

pub fn db_load_resp(game_entity: &mut GameSharedEntity, _vfd: u64, pto: ProtoType) -> Result<()> {
    let ptoobj = match &pto {
        ProtoType::db_load_resp(ptoobj) => ptoobj,
        _ => return Ok(()),
    };
    let ret_func = DBRetFuncMarker::from_u64(ptoobj.ret_func).into_func();
    if ret_func.is_none() {
        llog::info!(LOG_NAME, "[db_load_resp]: db_funcs: {}", ptoobj.ret_func);
        return Ok(());
    }
    let ret_func = ret_func.unwrap();
    ret_func(game_entity, pto);
    Ok(())
}
