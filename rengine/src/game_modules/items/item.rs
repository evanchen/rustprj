use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Item {
    uid: u64,
    id: u32,
    stack: i32,
}

impl Item {
    pub fn new(uid: u64, id: u32, stack: i32) -> Self {
        Item { uid, id, stack }
    }

    pub fn uid(&self) -> u64 {
        self.uid
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn stack(&self) -> i32 {
        self.stack
    }

    pub fn add_stack(&mut self, addnum: i32) -> i32 {
        self.stack += addnum;
        self.stack
    }

    pub fn log_info(&self) -> String {
        format!(
            "item_uid={},item_id={},stack={}",
            self.uid, self.id, self.stack
        )
    }
}
