use iron_exec::{config::Config, worker::Worker};

use super::logs::LOG_DIR;

pub struct TestWorker(pub Worker);

impl TestWorker {
    pub fn new() -> TestWorker {
        let cfg = Config::new(LOG_DIR);
        TestWorker(Worker::new(cfg))
    }
}
