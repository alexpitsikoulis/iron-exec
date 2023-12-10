use std::thread::{self, JoinHandle};

use iron_exec::{
    config::Config,
    job::{CgroupConfig, Command, Job, Status},
    worker::Worker,
};
use uuid::Uuid;

use super::logs::{TestLog, LOG_DIR};

pub struct TestApp {
    pub worker: Worker,
    pub log_handler: TestLog,
}

impl TestApp {
    pub fn new() -> TestApp {
        let cfg = Config::new(LOG_DIR);
        TestApp {
            worker: Worker::new(cfg),
            log_handler: TestLog::new(),
        }
    }

    #[allow(dead_code)]
    pub fn queue_job(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
    ) -> (Box<Job>, JoinHandle<()>) {
        let (job, job_handle) = self.worker.start(command, cgroup_config, Uuid::new_v4());

        let wait_handle = thread::spawn(move || {
            job_handle.join().unwrap().unwrap();
        });

        loop {
            if *job.status().lock().unwrap() == Status::Running {
                break;
            }
        }
        (job, wait_handle)
    }
}
