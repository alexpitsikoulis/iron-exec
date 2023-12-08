use iron_exec::{config::Config, worker::Worker};

use super::logs::LOG_DIR;

pub fn spawn() -> Worker {
    let cfg = Config::new(LOG_DIR);
    Worker::new(cfg)
}
