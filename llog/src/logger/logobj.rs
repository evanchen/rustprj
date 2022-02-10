use chrono::{Local, TimeZone};
use std::cmp::Eq;
use std::fs::File;
use std::hash::Hash;
use std::io::{Result, Write};

#[derive(Debug, PartialEq, PartialOrd, Eq, Hash, Clone, Copy)]
pub enum LevelType {
    Debug,
    Warning,
    Info,
    Error,
}

impl LevelType {
    pub fn from_i32(n: i32) -> Self {
        match n {
            1 => LevelType::Debug,
            2 => LevelType::Warning,
            3 => LevelType::Info,
            4 => LevelType::Error,
            _ => LevelType::Debug,
        }
    }
}

#[derive(Debug)]
pub struct Logger {
    _fname: String,
    path: String,
    create_date: i64,
    fh: File,
    fsize: u64,
    roll_num: u32,
}

impl Logger {
    pub fn new(fname: &str, path: &str, fh: File) -> Logger {
        Logger {
            _fname: fname.to_owned(),
            path: path.to_owned(),
            create_date: Local::today().and_hms_milli(0, 0, 0, 0).timestamp(),
            fh,
            fsize: 0,
            roll_num: 1,
        }
    }

    pub fn can_roll(&self, cur_date: i64, max_size: u64) -> bool {
        self.create_date != cur_date || self.fsize >= max_size
    }

    pub fn update_file_roll(&mut self, fh: File) {
        self.fh = fh;
        self.fsize = 0;
        self.create_date = Local::today().and_hms_milli(0, 0, 0, 0).timestamp();
    }

    pub fn get_path(&self) -> String {
        self.path.clone()
    }

    pub fn get_create_date(&self) -> String {
        let dt = Local.timestamp(self.create_date, 0);
        dt.format("%Y-%m-%d").to_string()
    }

    pub fn get_roll_num(&mut self) -> u32 {
        let res = self.roll_num;
        self.roll_num += 1;
        res
    }

    pub fn write(&mut self, datestr: &str, lvname: &str, logstr: &str) -> Result<()> {
        let wsize = logstr.len();
        writeln!(self.fh, "[{}][{}]: {}", datestr, lvname, logstr)?; //one line for each write
        self.fsize += wsize as u64;
        self.fh.flush()?;
        Ok(())
    }

    pub fn debug(&mut self, datestr: &str, cur_level: LevelType, logstr: &str) {
        if cur_level > LevelType::Debug {
            return;
        }
        self.dowrite(datestr, "debug", logstr)
    }

    pub fn warning(&mut self, datestr: &str, cur_level: LevelType, logstr: &str) {
        if cur_level > LevelType::Warning {
            return;
        }
        self.dowrite(datestr, "warn ", logstr)
    }

    pub fn info(&mut self, datestr: &str, cur_level: LevelType, logstr: &str) {
        if cur_level > LevelType::Info {
            return;
        }
        self.dowrite(datestr, "info ", logstr)
    }

    pub fn error(&mut self, datestr: &str, cur_level: LevelType, logstr: &str) {
        if cur_level > LevelType::Error {
            return;
        }
        self.dowrite(datestr, "error", logstr);
        println!("[error]: {}", logstr);
    }

    fn dowrite(&mut self, datestr: &str, lvname: &str, logstr: &str) {
        if let Err(err) = self.write(datestr, lvname, logstr) {
            println!("{:?}", err);
        }
    }
}
