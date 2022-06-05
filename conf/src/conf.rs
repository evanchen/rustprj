extern crate serde_derive;
extern crate toml;
use std::fs::File;
use std::io::prelude::*;
//use std::env;

/*  :TODO:
for now, every crate is sharing one Conf struct,but they owns their .toml file
maybe try to write a simple parser would be better( a hashmap).
*/
#[derive(serde_derive::Deserialize, Debug, Clone)]
pub struct Conf {
    host_id: u64,
    name: String,
    host_type: String,

    // llog conf
    log_level: i32,
    max_log_file_size: i32,

    // proto conf
    src_dir: String,
    out_dir: String,

    init_protos: Vec<String>,

    // tcp service
    tcp_serv_addr: String,

    // http service
    http_serv_addr: String,

    // rpc service
    rpc_serv_addr: String,

    // rpc db service
    db_host_id: u64,
    rpc_db_serv_addr: String,
}

impl Conf {
    pub fn new() -> Conf {
        //println!("{:?}",env::current_dir().unwrap());
        let fname = "conf/conf.toml";
        let mut file = match File::open(fname) {
            Ok(f) => f,
            Err(e) => panic!("open {} error: {}", fname, e),
        };
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(s) => s,
            Err(e) => panic!("read_to_string: {}", e),
        };
        //println!("[new]:contents {}", contents);
        let conf: Conf = toml::from_str(&contents).unwrap();
        //println!("[new]: {:?}", conf);
        if conf.get_host_type() == "db" {
            assert_eq!(conf.get_db_host_id(), 99999999);
            assert_eq!(conf.get_host_id(), conf.get_db_host_id());
            assert_eq!(conf.get_rpc_serv_addr(), conf.get_rpc_db_serv_addr());
        } else {
            assert_ne!(conf.get_host_id(), 99999999);
        }
        conf
    }

    pub fn get_host_id(&self) -> u64 {
        self.host_id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_host_type(&self) -> &str {
        &self.host_type
    }

    pub fn get_log_level(&self) -> i32 {
        self.log_level
    }

    pub fn get_log_file_size(&self) -> i32 {
        self.max_log_file_size
    }

    pub fn get_src_dir(&self) -> String {
        self.src_dir.clone()
    }

    pub fn get_out_dir(&self) -> String {
        self.out_dir.clone()
    }

    pub fn get_init_protos(&self) -> &Vec<String> {
        &self.init_protos
    }

    pub fn get_tcp_serv_addr(&self) -> &str {
        &self.tcp_serv_addr
    }

    pub fn get_http_serv_addr(&self) -> &str {
        &self.http_serv_addr
    }

    pub fn get_rpc_serv_addr(&self) -> &str {
        &self.rpc_serv_addr
    }

    pub fn get_db_host_id(&self) -> u64 {
        self.db_host_id
    }

    pub fn get_rpc_db_serv_addr(&self) -> &str {
        &self.rpc_db_serv_addr
    }
}

impl Default for Conf {
    fn default() -> Self {
        Self::new()
    }
}
