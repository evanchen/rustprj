use std::fmt::Debug;

use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug)]
pub struct MailBox<T> {
    pub send: Sender<T>,
    pub recv: Receiver<T>,
}

impl<T> MailBox<T> {
    pub fn new(bounded_size: usize) -> Self {
        let (send, recv) = mpsc::channel::<T>(bounded_size);
        MailBox { send, recv }
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.recv.recv().await
    }

    pub async fn send(&mut self, val: T) -> crate::Result<()>
    where
        T: Debug,
    {
        match self.send.send(val).await {
            Ok(_) => Ok(()),
            Err(err) => {
                let errstr = format!("mailbox send fail: {:?}", err.0);
                Err(errstr.into())
            }
        }
    }
}

pub fn mailbox<T>(bounded_size: usize) -> MailBox<T> {
    let (send, recv) = mpsc::channel::<T>(bounded_size);
    MailBox { send, recv }
}
