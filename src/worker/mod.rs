mod config;
mod joiner;
use crate::job::{Command, Job, Status};
pub use config::Config;
use crossbeam::channel::Receiver;
use joiner::Joiner;
use std::{
    fmt::Display,
    fs::File,
    os::fd::AsRawFd,
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum Error {
    WorkerErr(String),
    JobErr(Uuid, String),
    JobStartErr(String),
    JobStopErr(String),
    JobQueryErr(String),
    JobStreamErr(String),
}

impl Error {
    pub fn as_str(&self) -> &str {
        match self {
            Self::WorkerErr(e) => e,
            Self::JobErr(_, e) => e,
            Self::JobStartErr(e) => e,
            Self::JobStopErr(e) => e,
            Self::JobQueryErr(e) => e,
            Self::JobStreamErr(e) => e,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.as_str())
    }
}

impl std::error::Error for Error {}

#[derive(Clone)]
pub struct Worker {
    cfg: Config,
    joiner: Joiner,
    notify_receiver: Receiver<Result<Uuid, Error>>,
    pub jobs: Arc<Mutex<Vec<Box<Job>>>>,
}

impl Worker {
    pub fn new(cfg: Config) -> Result<Self, Error> {
        let (tx, rx) = crossbeam::channel::bounded(cfg.thread_count());
        let joiner = Joiner::new(cfg.thread_count(), tx);
        let worker = Worker {
            cfg,
            joiner,
            notify_receiver: rx,
            jobs: Arc::new(Mutex::new(vec![])),
        };
        worker
            .create_log_dir()
            .map_err(|e| Error::WorkerErr(format!("failed to create log directory: {:?}", e)))?;
        Ok(worker)
    }

    pub fn start(&self, command: Command, owner_id: Uuid) -> Result<Uuid, Error> {
        let job_id = Uuid::new_v4();

        let log_filepath =
            Path::new(self.cfg.log_dir()).join(format!("{}_{}.log", command.name(), job_id));
        let log_file = File::create(&log_filepath)
            .map_err(|e| Error::JobStartErr(format!("failed to create log file: {:?}", e)))?;

        let (job, child_proc) = match Job::start(job_id, command, owner_id, log_file.as_raw_fd()) {
            Ok((job, proc)) => (Box::new(job), proc),
            Err(e) => return Err(e),
        };
        let mut jobs = self.jobs.lock().unwrap();
        jobs.push(job.clone());

        self.joiner.queue_job(job.wait(child_proc));

        Ok(job_id)
    }

    pub fn stop(&self, job_id: Uuid, owner_id: Uuid, gracefully: bool) -> Result<(), Error> {
        match self.find_job(job_id, owner_id) {
            Some(job) => job.stop(gracefully),
            None => Err(Error::JobStopErr(format!(
                "no job with id {} found for user",
                job_id
            ))),
        }
    }

    pub fn query(&self, job_id: Uuid, owner_id: Uuid) -> Result<Status, Error> {
        match self.find_job(job_id, owner_id) {
            Some(job) => {
                let status = job
                    .status()
                    .lock()
                    .map_err(|e| {
                        Error::JobStartErr(format!(
                            "failed to lock status mutex for job {}: {:?}",
                            job.id(),
                            e
                        ))
                    })?
                    .clone();
                Ok(status)
            }
            None => Err(Error::JobQueryErr(format!(
                "no job with id {} found for user",
                job_id
            ))),
        }
    }

    pub fn stream(&self, job_id: Uuid, owner_id: Uuid) -> Result<std::io::BufReader<File>, Error> {
        match self.find_job(job_id, owner_id) {
            Some(job) => job.stream(self.cfg.log_dir()),
            None => Err(Error::JobStreamErr(format!(
                "no job with id {} found for user",
                job_id,
            ))),
        }
    }

    pub fn notify_receiver(&self) -> Receiver<Result<Uuid, Error>> {
        self.notify_receiver.clone()
    }

    fn find_job(&self, job_id: Uuid, owner_id: Uuid) -> Option<Box<Job>> {
        let jobs = self.jobs.lock().unwrap();
        jobs.clone()
            .iter()
            .find(|job| job.id() == job_id && job.owner_id() == owner_id)
            .map(|job| job.clone())
    }

    fn create_log_dir(&self) -> Result<(), Error> {
        let log_dir_path = self.cfg.log_dir();
        if std::fs::read_dir(log_dir_path).is_err() {
            std::fs::create_dir_all(log_dir_path)
                .map_err(|e| Error::JobStartErr(format!("failed to create log directory: {:?}", e)))
        } else {
            Ok(())
        }
    }
}
