use tokio::sync::{mpsc, oneshot};

pub mod http_service;

#[derive(Debug)]
pub enum HttpProtoType {
    ReqServerAll,
    ReqServerInfo(u32),
    RespServerInfo(String),
    Unimplemented(String),
}

pub type HttpProtoSenderOp = oneshot::Sender<HttpProtoType>;
pub type ProtoReceiverOp = oneshot::Receiver<HttpProtoType>;
pub type HttpProtoSenderMp = mpsc::Sender<HttpProtoType>;
pub type ProtoReceiverMp = mpsc::Receiver<HttpProtoType>;
pub type ChanHttpProtoSenderOp = mpsc::Sender<(HttpProtoType, HttpProtoSenderOp)>;
pub type ChanHttpProtoReceiverOp = mpsc::Receiver<(HttpProtoType, HttpProtoSenderOp)>;
