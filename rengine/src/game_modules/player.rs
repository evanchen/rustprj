use super::db::DBObj;
use super::items::item_mgr::ItemMgr;
use crate::{
    errors::Error,
    shared_states::{GameSharedEntity, RpcSharedEntity},
    Result,
};
use net::{utils, Communicate, ProtoSender, ProtoType};
use proto::ptoout::*;
use proto::util::default_random_value;
use serde::{Deserialize, Serialize};

const LOG_NAME: &str = "player.log";

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Player {
    counter: u64,
    vfd: u64,
    acc: String,
    uid: u64,
    name: String,
    level: i32,
    expr: i32,
    money: i32,
    #[serde(skip)]
    sender: Option<ProtoSender>,
    #[serde(skip)]
    magic: i32,
    item_mgr: ItemMgr,
    #[serde(skip)]
    inner: Option<DBObj>,
}

impl Player {
    pub fn new(acc: String, uid: u64, name: String) -> Self {
        Player {
            acc,
            uid,
            name,
            item_mgr: ItemMgr::new(uid),
            inner: None,
            ..Default::default()
        }
    }

    pub fn get_vfd(&self) -> u64 {
        self.vfd
    }

    pub fn get_acc(&self) -> &str {
        &self.acc
    }

    pub fn get_uid(&self) -> u64 {
        self.uid
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_level(&self) -> i32 {
        self.level
    }

    pub fn get_expr(&self) -> i32 {
        self.expr
    }

    pub fn get_money(&self) -> i32 {
        self.money
    }

    pub fn update_sender(&mut self, new_sender: ProtoSender) {
        self.sender = Some(new_sender);
    }

    pub fn get_sender(&mut self) -> &Option<ProtoSender> {
        return &self.sender;
    }

    pub fn set_magic(&mut self, magic: i32) {
        self.magic = magic;
    }

    pub fn get_item_mgr(&mut self) -> &mut ItemMgr {
        &mut self.item_mgr
    }

    pub fn send(&self, proto_id: u32, pto: ProtoType) {
        if self.sender.is_none() {
            println!("[sender]: none");
            return;
        }

        utils::try_send(
            LOG_NAME,
            self.sender.as_ref().unwrap(),
            self.vfd,
            proto_id,
            pto,
        );
    }

    pub fn load_ret(game_entity: &mut GameSharedEntity, pto: ProtoType) {
        let ptoobj = match pto {
            ProtoType::db_load_resp(ptoobj) => ptoobj,
            _ => return,
        };
        let vfd = ptoobj.vfd;
        match game_entity.get_vfd_info(vfd) {
            Some((uid, acc)) => {
                if *uid > 0 {
                    //已加载过
                    return;
                } else if !acc.eq(&ptoobj.key) {
                    return;
                }
            }
            None => return, // 已掉线
        };

        let ch = match game_entity.tcp_entity.get(vfd) {
            Some(ch) => ch.clone(),
            None => return,
        };

        // 初始化 player 对象
        let mut player: Player = match serde_json::from_slice(&ptoobj.value) {
            Ok(player) => player,
            Err(err) => {
                llog::error!(
                    LOG_NAME,
                    "[resp_db_load_account]: vfd={},acc={},err={:?}",
                    vfd,
                    ptoobj.key,
                    err
                );
                return;
            }
        };
        let magic = default_random_value("i32").parse().unwrap();
        player.update_sender(ch.clone());
        player.set_magic(magic);

        //告诉客户端登录加载完毕
        let sendptoid = c_login::c_login::id();
        let mut c_login = c_login::c_login::default();
        c_login.ret = 1;
        c_login.magic = magic;
        let sendpto = ProtoType::c_login(c_login);
        player.send(sendptoid, sendpto);

        game_entity.add_player(player);
    }

    pub fn save(&mut self, rpc_entity: &mut RpcSharedEntity) {
        self.counter += 1;
        let datastr = serde_json::to_vec(self).unwrap(); // shouldn't failed!
        self.inner
            .as_ref()
            .unwrap()
            .save(rpc_entity, self.counter, datastr)
    }
}

pub fn player_do_mut<F, T>(game_entity: &mut GameSharedEntity, uid: u64, mut f: F) -> Result<T>
where
    F: FnMut(&mut GameSharedEntity, &mut Player) -> Result<T>,
{
    if game_entity
        .player_by_uid
        .as_ref()
        .unwrap()
        .get(&uid)
        .is_none()
    {
        return Err(Error::Feedback((111, "no player".to_string())));
    }

    let mut allplayers = game_entity.player_by_uid.take().unwrap();
    let player = allplayers.get_mut(&uid).unwrap();
    let res = f(game_entity, player);
    game_entity.player_by_uid = Some(allplayers);
    res
}

pub trait Tplayer {
    fn get_player_by_uid(&mut self, uid: u64) -> Option<&mut Player>;
    fn get_player_by_vfd(&mut self, vfd: u64) -> Option<&mut Player>;
    fn add_uid_acc(&mut self, vfd: u64, uid: u64, acc: String);
    fn get_vfd_info(&self, vfd: u64) -> Option<&(u64, String)>;
    fn is_vfd_validated(&self, vfd: u64) -> bool;
    fn add_player(&mut self, player: Player);
    fn remove_player_by_vfd(&mut self, vfd: u64);
    fn remove_player_by_uid(&mut self, uid: u64);
    fn remove_player(&mut self, vfd: u64, uid: u64);
}

impl Tplayer for GameSharedEntity {
    fn get_player_by_uid(&mut self, uid: u64) -> Option<&mut Player> {
        self.player_by_uid.as_mut().unwrap().get_mut(&uid)
    }

    fn get_player_by_vfd(&mut self, vfd: u64) -> Option<&mut Player> {
        if let Some((uid, _acc)) = self.vfd2uidacc.get(&vfd) {
            self.player_by_uid.as_mut().unwrap().get_mut(&uid)
        } else {
            None
        }
    }

    fn add_uid_acc(&mut self, vfd: u64, uid: u64, acc: String) {
        self.vfd2uidacc.insert(vfd, (uid, acc));
    }

    fn get_vfd_info(&self, vfd: u64) -> Option<&(u64, String)> {
        self.vfd2uidacc.get(&vfd)
    }

    fn is_vfd_validated(&self, vfd: u64) -> bool {
        self.get_vfd_info(vfd).is_some()
    }

    fn add_player(&mut self, player: Player) {
        let uid = player.get_uid();
        let vfd = player.get_vfd();
        let acc = player.get_acc().to_string();
        self.player_by_uid
            .as_mut()
            .unwrap()
            .insert(player.get_uid(), player);
        self.vfd2uidacc.insert(vfd, (uid, acc));

        llog::info!(LOG_NAME, "[add_player]: vfd={},uid={}", vfd, uid);
    }

    fn remove_player_by_vfd(&mut self, vfd: u64) {
        self.vfd2uidacc.remove(&vfd);
    }

    fn remove_player_by_uid(&mut self, uid: u64) {
        self.player_by_uid.as_mut().unwrap().remove(&uid);
    }

    fn remove_player(&mut self, vfd: u64, uid: u64) {
        self.remove_player_by_vfd(vfd);
        self.remove_player_by_uid(uid);
    }
}
