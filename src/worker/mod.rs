mod config;
use crate::job::{Command, Job, JobInfo};
pub use config::Config;
use crossbeam::channel::{Receiver, Sender};
use std::{
    fmt::Display,
    fs::File,
    os::fd::AsRawFd,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use threadpool::ThreadPool;
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
    thread_pool: ThreadPool,
    notify_chan: (
        Sender<Result<(Uuid, bool), Error>>,
        Receiver<Result<(Uuid, bool), Error>>,
    ),
    pub jobs: Arc<Mutex<Vec<Box<Job>>>>,
}

impl Worker {
    pub fn new(cfg: Config) -> Result<Self, Error> {
        let (tx, rx) = crossbeam::channel::bounded(cfg.thread_count());
        let thread_pool = ThreadPool::new(cfg.thread_count());
        let worker = Worker {
            cfg,
            thread_pool,
            notify_chan: (tx, rx),
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

        let sender = self.notify_chan.0.clone();
        self.thread_pool.execute(move || {
            if let Err(e) = sender.send(job.wait(child_proc)) {
                panic!("failed to send job result from execution thread: {:?}", e);
            };
        });

        Ok(job_id)
    }

    pub fn stop(&self, job_id: Uuid, owner_id: Uuid, gracefully: bool) -> Result<(), Error> {
        let sender = self.notify_chan.0.clone();
        match self.find_job(job_id, owner_id) {
            Some(job) => job.stop(gracefully, sender),
            None => Err(Error::JobStopErr(format!(
                "no job with id {} found for user",
                job_id
            ))),
        }
    }

    pub fn query(&self, job_id: Uuid, owner_id: Uuid) -> Result<JobInfo, Error> {
        match self.find_job(job_id, owner_id) {
            Some(job) => job.query(),
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

    pub fn notify_receiver(&self) -> Receiver<Result<(Uuid, bool), Error>> {
        self.notify_chan.1.clone()
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

impl Drop for Worker {
    fn drop(&mut self) {
        let jobs = self.jobs.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(30));
            let jobs = jobs.lock().unwrap();
            let pids = jobs.iter().map(|job| job.pid());
            println!("hanging processes are preventing graceful shutdown of the worker, the following pids are responsible: {:?}", pids);
        });
        self.thread_pool.join();
    }
}
