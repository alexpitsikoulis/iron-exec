use iron_exec::worker::{Config, Worker};

use super::logs::{TestLog, LOG_DIR};

pub struct TestApp {
    pub worker: Worker,
    pub log_handler: TestLog,
}

impl TestApp {
    pub fn new() -> TestApp {
        let cfg = Config::new(LOG_DIR);
        TestApp {
            worker: Worker::new(cfg).unwrap(),
            log_handler: TestLog::new(),
        }
    }
}
