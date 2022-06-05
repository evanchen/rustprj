use crate::{ProtoSender, ProtoType};
use proto::ptoout::*;
use tokio::sync::mpsc::error::TrySendError;

// 必须是 try_send
// return=0,ok; 1,channel full; 2,channel closed
pub fn try_send<'a>(
    log_name: &'a str,
    sender: &ProtoSender,
    vfd: u64,
    proto_id: u32,
    pto: ProtoType,
) -> Option<(i32, ProtoType)> {
    if let Err(err) = sender.try_send((vfd, proto_id, pto)) {
        match err {
            TrySendError::Full(err) => {
                llog::error!(
                    log_name,
                    "[utils.send]: failed, chan full: vfd={},proto_id={}",
                    vfd,
                    proto_id
                );
                return Some((1, err.2));
            }
            TrySendError::Closed(err) => {
                llog::error!(log_name, "[utils.send]: failed channel close: vfd={}", vfd);
                return Some((2, err.2));
            }
        }
    }
    None
}

pub fn feekback<'a>(
    log_name: &'a str,
    sender: &ProtoSender,
    vfd: u64,
    feedback_id: u32,
    params: String,
) {
    let sendptoid = c_errors::c_errors::id();
    let c_errors = c_errors::c_errors {
        id: feedback_id,
        param: params,
    };
    let sendpto = ProtoType::c_errors(c_errors);
    let _ = try_send(log_name, sender, vfd, sendptoid, sendpto);
}
