use tokio::sync::mpsc::{Receiver, Sender};

pub mod http;
pub mod rpc;
pub mod tcp;

use proto::allptos::ProtoType;
// vfd,proto_id,pto
pub type ProtoMsgType = (u64, u32, ProtoType);

use tcp::{connection::Connection, mailbox::MailBox, shutdown::Shutdown};

#[derive(Debug)]
pub enum RecvType {
    FromSocket,
    FromService,
}

#[derive(Debug)]
pub enum ServiceType {
    Tcp,
    Rpc,
}

// for tcp proto
pub type ProtoSender = Sender<ProtoMsgType>;
pub type ProtoReceiver = Receiver<ProtoMsgType>;
pub type ChanProtoSender = Sender<(u64, ProtoSender)>;
pub type ChanProtoReceiver = Receiver<(u64, ProtoSender)>;
// for http proto

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ServiceState<S, T>
where
    S: Communicate<T>,
{
    pub entity: S,
    pub mailbox: MailBox<T>,
}

impl<S, T> ServiceState<S, T>
where
    S: Communicate<T>,
{
    pub fn new(entity: S, bounded_size: usize) -> Self {
        ServiceState {
            entity,
            mailbox: MailBox::new(bounded_size),
        }
    }
}

pub trait Communicate<T> {
    fn register(&mut self, vfd: u64, sender: Sender<T>);
    fn unregister(&mut self, vfd: u64);
    fn get(&mut self, vfd: u64) -> Option<&Sender<T>>;
}
