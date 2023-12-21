use iron_exec::worker::{Config, Error, Worker};
use uuid::Uuid;

use super::logs::{TestLog, LOG_DIR};

pub struct TestApp {
    pub worker: Worker,
    pub log_handler: TestLog,
}

impl TestApp {
    pub fn new() -> TestApp {
        let cfg = Config::new(LOG_DIR, 4);
        TestApp {
            worker: Worker::new(cfg).unwrap(),
            log_handler: TestLog::new(),
        }
    }

    pub fn wait(&self) -> Result<(Uuid, bool), Error> {
        let receiver = self.worker.notify_receiver();
        match receiver.recv() {
            Ok(res) => res,
            Err(e) => panic!("failed to receive job from joiner: {:?}", e),
        }
    }
}
