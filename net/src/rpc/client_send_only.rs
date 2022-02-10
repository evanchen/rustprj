use crate::{Connection, ProtoReceiver};
use proto::allptos;
use tokio::net::TcpStream;
extern crate llog;

#[derive(Debug)]
pub struct ClientSendOnly {
    identity: u64,
    connection: Connection,
    receiver: ProtoReceiver,
}

pub async fn start_service(
    addr: String,
    log_name: &'static str,
    identity: u64,
    receiver: ProtoReceiver,
) -> crate::Result<()> {
    // Establish a connection
    let socket = TcpStream::connect(addr).await.unwrap();
    let mut client = ClientSendOnly {
        identity,
        connection: Connection::new(socket, identity),
        receiver,
    };
    llog::info!(log_name, "new connection,identity={}", identity);

    tokio::select! {
        res = client.run(log_name) => {
            if let Err(err) = res {
                llog::error!(log_name,"[run]: error: {:?}",err);
                return Err(err);
            } else {
                llog::info!(log_name,"connection close,identity={}",identity);
            }
        }
    }
    Ok(())
}

impl ClientSendOnly {
    pub async fn run(&mut self, log_name: &'static str) -> crate::Result<()> {
        llog::info!(log_name, "[run]: accepting message");
        loop {
            match self.receiver.recv().await {
                Some((identity, proto_id, pto)) => {
                    if identity != self.identity {
                        llog::info!(
                            log_name,
                            "[run]: wrong identity={}, self.identity={}",
                            identity,
                            self.identity
                        );
                        return Err("wrong vfd".into());
                    }
                    //println!("cli send proto to socket: vfd={},proto_id={}", identity,proto_id);
                    let buf = allptos::serialize(pto)?;
                    self.connection.write_frame(proto_id, &buf).await?;
                }
                None => return Ok(()),
            };
        }
    }
}
