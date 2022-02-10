use super::mgr;

use chrono::Local;
use std::rc::Rc;

thread_local! {
    pub static LOGOBJ_MGR: Rc<mgr::LoggerMgr> = Rc::new(mgr::LoggerMgr::new());
}

pub fn can_log_debug() -> bool {
    LOGOBJ_MGR.with(|f| f.can_log_debug())
}
pub fn can_log_warning() -> bool {
    LOGOBJ_MGR.with(|f| f.can_log_warning())
}
pub fn can_log_info() -> bool {
    LOGOBJ_MGR.with(|f| f.can_log_info())
}
pub fn can_log_error() -> bool {
    LOGOBJ_MGR.with(|f| f.can_log_error())
}

pub fn debug(fname: &'static str, logstr: &str) {
    LOGOBJ_MGR.with(|f| {
        let lv = f.get_log_level();
        let datetime = Local::now();
        let datestr = datetime.format("%Y-%m-%d %H:%M:%S").to_string(); //datetime.to_string();
        if let Err(err) = f
            .get_logger(fname)
            .map(|rc| rc.borrow_mut().debug(&datestr, lv, logstr))
        {
            println!("[debug]: {:?}", err);
        } else {
            let cur_date = datetime.date().and_hms_milli(0, 0, 0, 0).timestamp();
            f.check_file_roll(fname, cur_date);
        }
    });
}

pub fn warning(fname: &'static str, logstr: &str) {
    LOGOBJ_MGR.with(|f| {
        let lv = f.get_log_level();
        let datetime = Local::now();
        let datestr = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        if let Err(err) = f
            .get_logger(fname)
            .map(|rc| rc.borrow_mut().warning(&datestr, lv, logstr))
        {
            println!("[warning]: {:?}", err);
        } else {
            let cur_date = datetime.date().and_hms_milli(0, 0, 0, 0).timestamp();
            f.check_file_roll(fname, cur_date);
        }
    })
}

pub fn info(fname: &'static str, logstr: &str) {
    LOGOBJ_MGR.with(|f| {
        let lv = f.get_log_level();
        let datetime = Local::now();
        let datestr = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        if let Err(err) = f
            .get_logger(fname)
            .map(|rc| rc.borrow_mut().info(&datestr, lv, logstr))
        {
            println!("[info]: {:?}", err);
        } else {
            let cur_date = datetime.date().and_hms_milli(0, 0, 0, 0).timestamp();
            f.check_file_roll(fname, cur_date);
        }
    })
}

pub fn error(fname: &'static str, logstr: &str) {
    LOGOBJ_MGR.with(|f| {
        let lv = f.get_log_level();
        let datetime = Local::now();
        let datestr = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        if let Err(err) = f
            .get_logger(fname)
            .map(|rc| rc.borrow_mut().error(&datestr, lv, logstr))
        {
            println!("[error]: {:?}", err);
        } else {
            let cur_date = datetime.date().and_hms_milli(0, 0, 0, 0).timestamp();
            f.check_file_roll(fname, cur_date);
        }
    })
}
