use crate::job::{Command, Job, Status, StopType};
use std::{
    fmt::Display,
    fs::File,
    io::BufReader,
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
    process::{Child, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use syscalls::{syscall, Sysno};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum Error {
    WorkerErr(String),
    JobErr(String),
    JobStartErr(String),
    JobStopErr(String),
    JobQueryErr(String),
    JobStreamErr(String),
}

impl Error {
    pub fn as_str(&self) -> &str {
        match self {
            Self::WorkerErr(e) => e,
            Self::JobErr(e) => e,
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

#[derive(Debug, Clone)]
pub struct Config {
    log_dir: &'static str,
}

impl Config {
    pub fn new(log_dir: &'static str) -> Self {
        Config { log_dir }
    }

    pub fn default() -> Self {
        Config { log_dir: "/tmp" }
    }
}

#[derive(Clone)]
pub struct Worker {
    cfg: Config,
    pub jobs: Vec<Box<Job>>,
}

impl Worker {
    pub fn new(cfg: Config) -> Result<Self, Error> {
        let worker = Worker { cfg, jobs: vec![] };
        worker
            .create_log_dir()
            .map_err(|e| Error::WorkerErr(format!("failed to create log directory: {:?}", e)))?;
        Ok(worker)
    }

    pub fn start(
        &mut self,
        command: Command,
        owner_id: Uuid,
    ) -> Result<(Uuid, JoinHandle<Result<(), Error>>), Error> {
        let job_id = Uuid::new_v4();
        let mut cmd = &mut std::process::Command::new(command.name());
        let log_filepath =
            Path::new(self.cfg.log_dir).join(format!("{}_{}.log", command.name(), job_id));
        let log_filepath_string = match log_filepath.to_str() {
            Some(path) => path,
            None => "/tmp",
        };
        let log_file = File::create(&log_filepath)
            .map_err(|e| Error::JobStartErr(format!("failed to create log file: {:?}", e)))?;

        unsafe {
            cmd = cmd
                .stdout(Stdio::from_raw_fd(log_file.as_raw_fd()))
                .stderr(Stdio::from_raw_fd(log_file.as_raw_fd()));
        }

        let child_cmd = cmd.args(command.args());
        let child_proc = match child_cmd.spawn() {
            Ok(child) => Arc::new(Mutex::new(child)),
            Err(e) => {
                std::fs::remove_file(log_filepath_string).expect(&format!("failed to remove log file {} on job startup failure, please remove it manually", log_filepath_string));
                return Err(Error::JobStartErr(format!(
                    "failed to spawn child process: {:?}",
                    e
                )));
            }
        };

        let status = Arc::new(Mutex::new(Status::UnknownState));

        *status.lock().map_err(|e| {
            Error::JobStartErr(format!(
                "failed to lock status mutex for job {}: {:?}",
                job_id, e
            ))
        })? = Status::Running;

        self.jobs.push(Box::new(Job::new(
            job_id,
            command,
            match child_proc.lock() {
                Ok(proc) => proc.id(),
                Err(e) => {
                    return Err(Error::JobStartErr(format!(
                        "failed to lock child process mutex to access pid: {:?}",
                        e
                    )))
                }
            },
            status.clone(),
            owner_id,
        )));

        let wait_thread = thread::spawn(|| wait(status, child_proc));
        Ok((job_id, wait_thread))
    }

    pub fn stop(&mut self, job_id: Uuid, owner_id: Uuid, gracefully: bool) -> Result<(), Error> {
        match self.find_job(job_id, owner_id) {
            Some(job) => {
                let pid = job.pid();
                let stop_type = match gracefully {
                    true => StopType::Term,
                    false => StopType::Kill,
                };
                unsafe {
                    match syscall!(Sysno::kill, pid, stop_type.sig()) {
                        Ok(_) => {
                            *job.status().lock().map_err(|e| {
                                Error::JobStartErr(format!(
                                    "failed to lock status mutex for job {}: {:?}",
                                    job.id(),
                                    e
                                ))
                            })? = Status::Stopped(stop_type);
                            Ok(())
                        }
                        Err(e) => Err(Error::JobStopErr(format!(
                            "failed to send SIG{} to job: {:?}",
                            stop_type.as_str().to_uppercase(),
                            e
                        ))),
                    }
                }
            }
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
            Some(job) => {
                match std::fs::File::open(format!(
                    "{}/{}_{}.log",
                    self.cfg.log_dir,
                    job.cmd().name(),
                    job.id()
                )) {
                    Ok(log_file) => Ok(BufReader::new(log_file)),
                    Err(e) => Err(Error::JobStreamErr(format!(
                        "failed to open log file: {:?}",
                        e
                    ))),
                }
            }
            None => Err(Error::JobStreamErr(format!(
                "no job with id {} found for user",
                job_id,
            ))),
        }
    }

    fn find_job(&self, job_id: Uuid, owner_id: Uuid) -> Option<&Box<Job>> {
        self.jobs
            .iter()
            .find(|job| job.id() == job_id && job.owner_id() == owner_id)
    }

    fn create_log_dir(&self) -> Result<(), Error> {
        let log_dir_path = self.cfg.log_dir;
        if std::fs::read_dir(log_dir_path).is_err() {
            std::fs::create_dir_all(log_dir_path)
                .map_err(|e| Error::JobStartErr(format!("failed to create log directory: {:?}", e)))
        } else {
            Ok(())
        }
    }
}

fn wait(status: Arc<Mutex<Status>>, proc: Arc<Mutex<Child>>) -> Result<(), Error> {
    let proc_lock = match Arc::try_unwrap(proc) {
        Ok(mtx) => mtx,
        Err(e) => {
            return Err(Error::JobErr(format!(
                "failed to unwrap child process ARC: {:?}",
                e
            )))
        }
    };
    let inner = match proc_lock.into_inner() {
        Ok(inner) => inner,
        Err(e) => {
            return Err(Error::JobErr(format!(
                "failed to pull inner value from child process mutex: {:?}",
                e
            )))
        }
    };
    let output = match inner.wait_with_output() {
        Ok(out) => out,
        Err(e) => return Err(Error::JobErr(format!("child process failed: {:?}", e))),
    };
    match status.lock() {
        Ok(mut status) => {
            if !status.is_stopped() {
                *status = Status::Exited(output.status.code())
            }
        }
        Err(e) => {
            return Err(Error::JobErr(format!(
                "failed to lock status mutex to update exit code: {:?}",
                e
            )))
        }
    }
    Ok(())
}
