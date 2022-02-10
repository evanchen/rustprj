#[macro_use]
extern crate llog;
use std::thread;

#[test]
fn testwrite() {
    let thread_num = 1;
    let mut ts = vec![];
    for _ in 0..thread_num {
        let th = thread::spawn(|| {
            let fname = "player.log";
            for _n in 0..10000 {
                debug!(fname, "hahaxxx-{},{}", 123, "xss");
                warning!(fname, "hahaxxx-{},{}", 123, "xss");
                info!(fname, "hahaxxx-{},{}", 123, "xss");
                error!(fname, "hahaxxx-{},{}", 123, "xss");

                debug!(fname, "hahaxxx-{},{}", 123, "xss");
                warning!(fname, "hahaxxx-{},{}", 123, "xss");
            }
        });
        ts.push(th);
    }

    for th in ts {
        th.join().unwrap();
    }
}
