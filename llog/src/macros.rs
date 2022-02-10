// debug_log!(logname,fmt,var1,var2,...)
#[macro_export]
macro_rules! debug {
    ($logname:expr,$($arg:tt)*) => {
        if $crate::logger::local::can_log_debug() {
            let str = format!($($arg)*);
            $crate::logger::local::debug($logname,&str);
        }
    };
}

#[macro_export]
macro_rules! warning {
    ($logname:expr,$($arg:tt)*) => {
        if $crate::logger::local::can_log_warning() {
            let str = format!($($arg)*);
            $crate::logger::local::warning($logname,&str);
        }
    };
}

#[macro_export]
macro_rules! info {
    ($logname:expr,$($arg:tt)*) => {
        if $crate::logger::local::can_log_info() {
            let str = format!($($arg)*);
            $crate::logger::local::info($logname,&str);
        }
    };
}

#[macro_export]
macro_rules! error {
    ($logname:expr,$($arg:tt)*) => {
        if $crate::logger::local::can_log_error() {
            let str = format!($($arg)*);
            $crate::logger::local::error($logname,&str);
        }
    };
}
