use super::{http_state, HttpSharedEntity, OperationEntity, RpcSharedEntity, TcpSharedEntity};
use crate::{
    errors::Error,
    game_modules::{
        player::{Player, Tplayer},
        uuid::{Tuuid, UUID},
    },
    proto_handlers::{ProtoRetFuncMarker, RpcRetFuncMarker},
    Result,
};
use conf::conf::Conf;
use net::{
    http::{HttpProtoSenderOp, HttpProtoType},
    utils, ProtoType,
};
use std::collections::HashMap;

const LOG_NAME: &str = "game_state.log";

pub struct GameSharedEntity {
    sysconf: Conf,
    pub op_entity: OperationEntity,
    pub tcp_entity: TcpSharedEntity,
    pub rpc_entity: RpcSharedEntity,
    pub http_entity: HttpSharedEntity,
    pub player_by_uid: Option<HashMap<u64, Player>>,
    pub vfd2uidacc: HashMap<u64, (u64, String)>,
    pub uuid: Option<UUID>,
    proto_need_not_vfd_validate: HashMap<String, bool>,
}

impl GameSharedEntity {
    pub fn new(
        sysconf: Conf,
        tcp_entity: TcpSharedEntity,
        rpc_entity: RpcSharedEntity,
        http_entity: HttpSharedEntity,
    ) -> Self {
        let host_id = sysconf.get_host_id();
        let mut game_entity = GameSharedEntity {
            sysconf,
            tcp_entity,
            rpc_entity,
            http_entity,
            player_by_uid: Some(HashMap::new()),
            vfd2uidacc: HashMap::new(),
            op_entity: Default::default(),
            uuid: None,
            proto_need_not_vfd_validate: HashMap::new(),
        };
        game_entity.uuid_init(host_id);
        game_entity
            .proto_need_not_vfd_validate
            .insert("s_login".to_string(), true);
        game_entity
    }

    // get / set methods
    pub fn get_system_conf(&self) -> &Conf {
        &self.sysconf
    }

    pub fn get_host_id(&self) -> u64 {
        self.sysconf.get_host_id()
    }

    pub async fn dispatch_tcp_msg(
        &mut self,
        vfd: u64,
        proto_id: u32,
        pto: ProtoType,
    ) -> Result<()> {
        if self.uuid.is_none() {
            println!(
                "[dispatch_tcp_msg]: vfd={},proto_id:={}, service_not_ready",
                vfd, proto_id
            );
            return Ok(());
        }

        let (pid, proto_name) = pto.inner_info();
        let proto_func = ProtoRetFuncMarker::from_str(proto_name).into_func();
        if proto_func.is_none() {
            llog::info!(
                LOG_NAME,
                "[tcp.dispatch_tcp_msg]: protocol id does not match: {},{},{}",
                proto_id,
                pid,
                proto_name
            );
            return Ok(());
        }

        if self.proto_need_not_vfd_validate.get(proto_name).is_none() {
            if !self.is_vfd_validated(vfd) {
                println!("[dispatch_tcp_msg]: vfd={} hasn't validated", vfd);
                return Ok(());
            }
        }
        let proto_func = proto_func.unwrap();
        if let Err(Error::Feedback((id, err))) = proto_func(self, vfd, pto) {
            self.get_player_by_vfd(vfd).map(|player| {
                if let Some(ch) = player.get_sender() {
                    utils::feekback(LOG_NAME, ch, vfd, id, err);
                }
            });
        }
        Ok(())
    }

    pub async fn dispatch_rpc_msg(
        &mut self,
        vfd: u64,
        proto_id: u32,
        pto: ProtoType,
    ) -> Result<()> {
        let (pid, proto_name) = pto.inner_info();
        let proto_func = RpcRetFuncMarker::from_str(proto_name).into_func();
        if proto_func.is_none() {
            llog::info!(
                LOG_NAME,
                "[tcp.dispatch_rpc_msg]: protocol id does not match: {},{},{}",
                proto_id,
                pid,
                proto_name
            );
            return Ok(());
        }
        let proto_func = proto_func.unwrap();
        if let Err(err) = proto_func(self, vfd, pto) {
            llog::info!(LOG_NAME, "[tcp.dispatch_rpc_msg]: {}", err);
        }
        Ok(())
    }

    pub async fn dispatch_http_msg(
        &mut self,
        hpt: HttpProtoType,
        optx: HttpProtoSenderOp,
    ) -> Result<()> {
        http_state::dispatch_http_msg(self, hpt, optx);
        Ok(())
    }
}
