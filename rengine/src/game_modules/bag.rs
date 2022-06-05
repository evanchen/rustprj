use super::items::Item;
use crate::errors::Error;
use crate::Result;
use proto::item_info::item_info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub enum BagType {
    Equiped = 1,
    Items = 2,
    Temp = 3,
}

impl BagType {
    pub fn into_u8(&self) -> u8 {
        match self {
            BagType::Equiped => 1,
            BagType::Items => 2,
            BagType::Temp => 3,
        }
    }

    pub fn from_u8(bag_type: u8) -> Self {
        match bag_type {
            1 => BagType::Equiped,
            2 => BagType::Items,
            3 => BagType::Temp,
            _ => BagType::Temp,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Bag {
    id: u8, // 1: equiped; 2, items; 3, temp. :TODO: 应该使用枚举变量,但枚举需要被 derive(Default),这是不想要的
    capacity: usize,
    inner: HashMap<u64, Item>, // [uid]=item or [equip_pos]=item
}

impl Bag {
    pub fn new(id: BagType, capacity: usize) -> Self {
        Bag {
            id: id.into_u8(),
            capacity,
            ..Default::default()
        }
    }

    pub fn add_item(&mut self, item: Item) -> Result<()> {
        if self.inner.len() <= self.capacity {
            return Err(Error::Message(format!("cap=failed,{}", item.log_info())));
        }
        self.inner.insert(item.uid(), item);
        Ok(())
    }

    pub fn del_item(&mut self, item_uid: u64) {
        self.inner.remove(&item_uid);
    }

    pub fn empty_space(&self) -> usize {
        self.capacity - self.inner.len()
    }

    pub fn get_item(&mut self, item_uid: u64) -> Option<&Item> {
        self.inner.get(&item_uid)
    }

    pub fn get_item_mut(&mut self, item_uid: u64) -> Option<&mut Item> {
        self.inner.get_mut(&item_uid)
    }

    pub fn pack(&self) -> Vec<item_info> {
        self.inner
            .iter()
            .map(|(uid, item)| item_info {
                uid: *uid,
                id: item.id(),
                stack: item.stack(),
            })
            .collect()
    }
}
