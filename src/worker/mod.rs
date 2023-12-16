use crate::{
    config::Config,
    job::{CgroupConfig, Command, Job, Status, StopType},
};
use nix::{
    sys::wait::waitpid,
    unistd::{execv, fork, getpid, ForkResult},
};
use std::{
    ffi::CString,
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
    JobStartErr(String),
    JobStopErr(String),
    JobQueryErr(String),
    JobStreamErr(String),
}

impl Error {
    pub fn as_str(&self) -> &str {
        match self {
            Self::WorkerErr(e) => e,
            Self::JobStartErr(e) => e,
            Self::JobStopErr(e) => e,
            Self::JobQueryErr(e) => e,
            Self::JobStreamErr(e) => e,
        }
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
        cgroup_config: Option<CgroupConfig>,
        owner_id: Uuid,
    ) -> Result<(Box<Job>, JoinHandle<()>), Error> {
        let job_id = Uuid::new_v4();
        let mut cmd = &mut std::process::Command::new(command.name());
        let log_filepath =
            Path::new(self.cfg.log_dir).join(format!("{}_{}.log", command.name(), job_id));
        let log_filepath_string = log_filepath.to_str().unwrap();
        let log_file = File::create(&log_filepath)
            .map_err(|e| Error::JobStartErr(format!("failed to create log file: {:?}", e)))?;

        unsafe {
            cmd = cmd
                .stdout(Stdio::from_raw_fd(log_file.as_raw_fd()))
                .stderr(Stdio::from_raw_fd(log_file.as_raw_fd()));
        }

        let child_proc = match cmd.args(command.args()).spawn() {
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
        let job = Box::new(Job::new(
            job_id,
            command,
            child_proc
                .lock()
                .map_err(|e| {
                    Error::JobStartErr(format!(
                        "failed to lock status mutex for job {}: {:?}",
                        job_id, e
                    ))
                })?
                .id(),
            status.clone(),
            owner_id,
        ));

        self.jobs.push(job.clone());

        *status.lock().map_err(|e| {
            Error::JobStartErr(format!(
                "failed to lock status mutex for job {}: {:?}",
                job_id, e
            ))
        })? = Status::Running;

        match cgroup_config {
            Some(config) => match unsafe { fork() } {
                Ok(ForkResult::Parent { child, .. }) => {
                    if let Err(e) = waitpid(child, None) {
                        child_proc
                            .lock()
                            .map_err(|e| {
                                Error::JobStartErr(format!(
                                    "failed to lock child process mutex for job {}: {:?}",
                                    job.id(),
                                    e
                                ))
                            })?
                            .kill()
                            .expect("failed to kill process after waitpid failed");
                        return Err(Error::JobStartErr(format!(
                            "waitpid failed for process with pid {}: {:?}",
                            child, e
                        )));
                    }
                }
                Ok(ForkResult::Child) => match config.init(job.clone()) {
                    Ok(cgroup_path) => {
                        let pid = getpid().as_raw() as u32;
                        if let Err(e) = config.add_process(cgroup_path, pid) {
                            child_proc.lock().map_err(|e| Error::JobStartErr(format!("failed to lock child process mutex for job {}: {:?}", job.id(), e)))?.kill().expect("failed to kill process after it failed to be added to the cgroup");
                            return Err(Error::JobStartErr(format!(
                                "add_to_cgroup failed: {:?}",
                                e
                            )));
                        }

                        let cmd = CString::new(command.name()).map_err(|e| {
                            Error::JobStartErr(format!(
                                "failed to generate cstring from command name: {:?}",
                                e
                            ))
                        })?;
                        let mut args: Vec<CString> = Vec::new();
                        for arg in command.args() {
                            args.push(CString::new(*arg).map_err(|e| {
                                Error::JobStartErr(format!(
                                    "failed to generate cstring from command name: {:?}",
                                    e
                                ))
                            })?)
                        }
                        match execv(&cmd, &args) {
                            Ok(_) => {}
                            Err(e) => {
                                return Err(Error::JobStartErr(format!(
                                    "failed to execute child process: {:?}",
                                    e,
                                )))
                            }
                        }
                    }
                    Err(e) => {
                        return Err(Error::JobStartErr(format!(
                            "failed to create cgroup: {:?}",
                            e
                        )))
                    }
                },
                Err(e) => {
                    return Err(Error::JobStartErr(format!(
                        "failed to fork process: {:?}",
                        e
                    )))
                }
            },
            None => {}
        }
        let wait_thread = thread::spawn(|| wait(status, child_proc));
        Ok((job.clone(), wait_thread))
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

fn wait(status: Arc<Mutex<Status>>, proc: Arc<Mutex<Child>>) {
    let proc_lock = Arc::try_unwrap(proc).unwrap();
    let inner = proc_lock.into_inner().unwrap();
    let output = inner.wait_with_output().unwrap();
    if !status.lock().unwrap().clone().is_stopped() {
        *status.lock().unwrap() = Status::Exited(output.status.code());
    }
}
