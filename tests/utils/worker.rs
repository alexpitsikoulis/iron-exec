use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use iron_exec::{
    config::Config,
    job::{CgroupConfig, Command, Job, Status},
    worker::{Error, Worker},
};
use uuid::Uuid;

use super::logs::LOG_DIR;

pub struct TestWorker(pub Worker);

impl TestWorker {
    pub fn new() -> TestWorker {
        let cfg = Config::new(LOG_DIR);
        TestWorker(Worker::new(cfg))
    }

    pub fn queue_job(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
    ) -> (Box<Job>, JoinHandle<()>) {
        let (job, job_handle) = self.0.start(command, cgroup_config, Uuid::new_v4());

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
