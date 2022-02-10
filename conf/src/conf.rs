extern crate serde_derive;
extern crate toml;
use std::fs::File;
use std::io::prelude::*;
//use std::env;

/*  :TODO:
for now, every crate is sharing one Conf struct,but they owns their .toml file
maybe try to write a simple parser would be better( a hashmap).
*/
#[derive(serde_derive::Deserialize, Debug)]
pub struct Conf {
    // llog conf
    log_level: i32,
    max_log_file_size: i32,

    // proto conf
    src_dir: String,
    out_dir: String,

    init_protos: Vec<String>,

    // tcp service
    tcp_port: i32,

    // http service
    http_port: i32,
}

impl Conf {
    pub fn new() -> Conf {
        //println!("{:?}",env::current_dir().unwrap());
        let fname = "../conf/conf.toml";
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
        conf
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

    pub fn get_tcp_port(&self) -> i32 {
        self.tcp_port
    }

    pub fn get_http_port(&self) -> i32 {
        self.http_port
    }
}

impl Default for Conf {
    fn default() -> Self {
        Self::new()
    }
}
