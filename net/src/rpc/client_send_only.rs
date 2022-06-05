use crate::{ConnWriter, ProtoReceiver};
use tokio::net::TcpStream;
extern crate llog;

pub async fn start_service<'a>(
    stream: TcpStream,
    log_name: &'a str,
    identity: u64,
    proto_rx: ProtoReceiver,
) -> crate::Result<()> {
    let (_read_stream, write_stream) = stream.into_split();

    let mut writer = ConnWriter::new(identity, write_stream, proto_rx);
    llog::info!(log_name, "new connection,identity={}", identity);

    if let Err(err) = writer.run(log_name).await {
        llog::error!(
            log_name,
            "[ConnWriter]: error: identity={},{:?}",
            identity,
            err
        );
        return Err(err);
    } else {
        llog::error!(log_name, "[ConnWriter]: return,identity={}", identity);
    }
    Ok(())
}
