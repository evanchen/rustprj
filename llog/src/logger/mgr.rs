use super::logobj::{LevelType, Logger};
use conf::conf;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Result;
use std::rc::Rc;
use std::{env, process, thread};

type RcLogType = Rc<RefCell<Logger>>;

pub struct LoggerMgr {
    log_level: LevelType,
    roll_file_size: u64,
    loggers: RefCell<HashMap<&'static str, RcLogType>>,
}

impl LoggerMgr {
    pub fn new() -> LoggerMgr {
        let sysconf = conf::Conf::new();
        let log_level = LevelType::from_i32(sysconf.get_log_level());
        let roll_file_size = (sysconf.get_log_file_size() as u64) * 1000 * 1000; // MBytes
        LoggerMgr {
            log_level,
            roll_file_size,
            loggers: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_log_level(&self) -> LevelType {
        self.log_level
    }
    pub fn can_log_debug(&self) -> bool {
        LevelType::Debug >= self.log_level
    }
    pub fn can_log_warning(&self) -> bool {
        LevelType::Warning >= self.log_level
    }
    pub fn can_log_info(&self) -> bool {
        LevelType::Info >= self.log_level
    }
    pub fn can_log_error(&self) -> bool {
        LevelType::Error >= self.log_level
    }

    pub fn get_logger(&self, fname: &'static str) -> Result<RcLogType> {
        if let Some(lg) = self.loggers.borrow().get(fname) {
            return Ok(lg.clone());
        }
        let path = self.filename2abs_path(fname);
        //create the path directory
        let pos = path.rfind('/').unwrap();
        let (dir, _) = path.split_at(pos);
        std::fs::create_dir_all(dir).unwrap();

        match OpenOptions::new()
            .append(true)
            .create(true)
            .open(path.clone())
        {
            Ok(fh) => {
                let res = Rc::new(RefCell::new(Logger::new(fname, &path, fh)));
                self.loggers.borrow_mut().insert(fname, res.clone());
                Ok(res)
            }
            Err(err) => Err(err),
        }
    }

    pub fn check_file_roll(&self, fname: &str, cur_date: i64) -> bool {
        if let Some(lg) = self.loggers.borrow().get(fname) {
            if !lg.borrow().can_roll(cur_date, self.roll_file_size) {
                return false;
            }
            let path = lg.borrow().get_path();
            let old_cur_date = lg.borrow().get_create_date();
            let rename = format!(
                "{}.{}_{}",
                path,
                old_cur_date,
                lg.borrow_mut().get_roll_num()
            );
            //note: rename first, but the fh has not been released until create a new fh
            if let Err(err) = std::fs::rename(lg.borrow().get_path(), rename) {
                println!("[check_file_roll]: {},{}", fname, err);
                return false;
            }

            match OpenOptions::new().append(true).create(true).open(path) {
                Ok(fh) => {
                    lg.borrow_mut().update_file_roll(fh);
                    return true;
                }
                Err(err) => {
                    println!("[check_file_roll]: {},{}", fname, err);
                    return false;
                }
            }
        }
        false
    }

    fn filename2abs_path(&self, fname: &str) -> String {
        let curdir = env::current_dir().unwrap();
        // for example, "xxx/log/player_20212_threadidxxx.log"
        format!(
            "{}/log/{}_{}_{:?}.log",
            curdir.to_str().unwrap(),
            fname,
            process::id(),
            thread::current().id()
        )
    }
}

impl Default for LoggerMgr {
    fn default() -> Self {
        Self::new()
    }
}
