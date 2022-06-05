use std::collections::HashMap;
use std::time::{Duration, Instant};

pub enum OperationStatus {
    Idle,
    Busy,
}

#[derive(Debug, Default)]
pub struct OperationEntity {
    op: HashMap<String, Instant>,
}

impl OperationEntity {
    pub fn can_start_op(&mut self, ukey: String) -> bool {
        match self.status(&ukey) {
            OperationStatus::Busy => false,
            _ => {
                let now = Instant::now();
                self.op.insert(ukey, now);
                true
            }
        }
    }

    pub fn finish_op(&mut self, ukey: &str) {
        self.op.remove(ukey);
    }

    fn status(&self, ukey: &str) -> OperationStatus {
        match self.op.get(ukey) {
            None => OperationStatus::Idle,
            Some(last_time) => {
                let duration = Duration::from_secs(60); // 60秒就算超时
                if last_time.elapsed() >= duration {
                    return OperationStatus::Idle;
                } else {
                    return OperationStatus::Busy;
                }
            }
        }
    }
}
