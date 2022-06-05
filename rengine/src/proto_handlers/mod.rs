use crate::shared_states::{DbSharedEntity, GameSharedEntity};
use crate::Result;
use net::ProtoType;

pub type ProtoHandlerFunc = fn(&mut GameSharedEntity, u64, ProtoType) -> Result<()>;
pub type DbProtoHandlerFunc = fn(&mut DbSharedEntity, u64, ProtoType) -> Result<()>;

pub mod item_handler;
pub mod leveldb_handler;
pub mod login_handler;
pub mod mysql_handler;
pub mod player_handler;
pub mod rpc_handler;

use item_handler::*;
use leveldb_handler::*;
use login_handler::*;
use player_handler::*;

// rpc 通信协议与协议处理函数的映射
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum RpcRetFuncMarker {
    //与 db 进程相关的
    db_load_resp,
    // 其他进程通信相关的
    unknow,
}

impl RpcRetFuncMarker {
    pub fn into_func(self) -> Option<ProtoHandlerFunc> {
        match self {
            RpcRetFuncMarker::db_load_resp => Some(db_load_resp),
            _ => None,
        }
    }

    pub fn from_str(proto_name: &str) -> Self {
        match proto_name {
            "db_load_resp" => RpcRetFuncMarker::db_load_resp,
            _ => RpcRetFuncMarker::unknow,
        }
    }
}

// rpc 通信协议与协议处理函数的映射
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum RpcDbSendFuncMarker {
    db_load_req,
    db_save_req,
    unknow,
}

impl RpcDbSendFuncMarker {
    pub fn into_func(self) -> Option<DbProtoHandlerFunc> {
        match self {
            RpcDbSendFuncMarker::db_load_req => Some(db_load_req),
            RpcDbSendFuncMarker::db_save_req => Some(db_save_req),
            _ => None,
        }
    }

    pub fn from_str(proto_name: &str) -> Self {
        match proto_name {
            "db_load_req" => RpcDbSendFuncMarker::db_load_req,
            "db_save_req" => RpcDbSendFuncMarker::db_save_req,
            _ => RpcDbSendFuncMarker::unknow,
        }
    }
}

// tcp 通信协议与协议处理函数的映射
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum ProtoRetFuncMarker {
    s_login,
    s_player_brief,
    s_item_bag,
    unknow,
}

impl ProtoRetFuncMarker {
    pub fn into_func(self) -> Option<ProtoHandlerFunc> {
        match self {
            ProtoRetFuncMarker::s_login => Some(s_login),
            ProtoRetFuncMarker::s_player_brief => Some(s_player_brief),
            ProtoRetFuncMarker::s_item_bag => Some(s_item_bag),
            _ => None,
        }
    }

    pub fn from_str(proto_name: &str) -> Self {
        match proto_name {
            "s_login" => ProtoRetFuncMarker::s_login,
            "s_player_brief" => ProtoRetFuncMarker::s_player_brief,
            "s_item_bag" => ProtoRetFuncMarker::s_item_bag,
            _ => ProtoRetFuncMarker::unknow,
        }
    }
}
