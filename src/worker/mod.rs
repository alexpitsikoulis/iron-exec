use crate::{
    config::Config,
    job::{add_to_cgroup, create_cgroup, CgroupConfig, Command, Job, Status, StopType},
};
use nix::{
    sys::wait::waitpid,
    unistd::{execv, fork, getpid, ForkResult},
};
use std::{
    ffi::CString,
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    process::{Child, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum Error {
    JobStartErr(String),
    JobStopErr(String),
    JobQueryErr(String),
    JobStreamErr(String),
}

#[derive(Clone)]
pub struct Worker {
    cfg: Config,
    pub jobs: Vec<Box<Job>>,
}

impl Worker {
    pub fn new(cfg: Config) -> Self {
        let worker = Worker { cfg, jobs: vec![] };
        worker.create_log_dir().unwrap();
        worker
    }

    pub fn start(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
        owner_id: Uuid,
    ) -> (Box<Job>, JoinHandle<Result<(), Error>>) {
        let job_id = Uuid::new_v4();
        let mut cmd = &mut std::process::Command::new(command.name());
        let log_file = File::create(format!(
            "{}/{}_{}.log",
            self.cfg.log_dir,
            command.name(),
            job_id,
        ))
        .unwrap();
        unsafe {
            cmd = cmd
                .stdout(Stdio::from_raw_fd(log_file.as_raw_fd()))
                .stderr(Stdio::from_raw_fd(log_file.as_raw_fd()));
        }
        let child_proc = Arc::new(Mutex::new(cmd.args(command.args()).spawn().unwrap()));

        let status = Arc::new(Mutex::new(Status::UnknownState));
        let job = Box::new(Job::new(
            job_id,
            command,
            child_proc.lock().unwrap().id(),
            status.clone(),
            owner_id,
        ));
        self.jobs.push(job.clone());
        let job_thread = thread::spawn(move || {
            *status.lock().unwrap() = Status::Running;
            match cgroup_config {
                Some(config) => match unsafe { fork() } {
                    Ok(ForkResult::Parent { child, .. }) => {
                        if let Err(e) = waitpid(child, None) {
                            child_proc
                                .lock()
                                .unwrap()
                                .kill()
                                .expect("failed to kill process after waitpid failed");
                            panic!("waitpid failed: {:?}", e);
                        }
                        wait(status, child_proc);
                        Ok(())
                    }
                    Ok(ForkResult::Child) => match create_cgroup(command.name(), job_id, config) {
                        Ok(cgroup_path) => {
                            let pid = getpid().as_raw() as u32;
                            if let Err(e) = add_to_cgroup(pid, cgroup_path) {
                                child_proc.lock().unwrap().kill().expect("failed to kill process after it failed to be added to the cgroup");
                                panic!("add_to_cgroup failed: {:?}", e);
                            }

                            let cmd = CString::new(command.name()).unwrap();
                            let mut args: Vec<CString> = Vec::new();
                            for arg in command.args() {
                                args.push(CString::new(*arg).unwrap())
                            }
                            match execv(&cmd, &args) {
                                Ok(_) => {
                                    wait(status, child_proc);
                                    Ok(())
                                }
                                Err(e) => Err(Error::JobStartErr(format!(
                                    "failed to execute child process: {:?}",
                                    e
                                ))),
                            }
                        }
                        Err(e) => Err(Error::JobStartErr(format!(
                            "failed to create cgroup: {:?}",
                            e
                        ))),
                    },
                    Err(e) => Err(Error::JobStartErr(format!(
                        "failed to fork process: {:?}",
                        e
                    ))),
                },
                None => {
                    wait(status, child_proc);
                    Ok(())
                }
            }
        });
        (job.clone(), job_thread)
    }

    pub fn stop(&mut self, job_id: Uuid, owner_id: Uuid, gracefully: bool) -> Result<(), Error> {
        match self
            .jobs
            .iter()
            .find(|job| job.id() == job_id && job.owner_id() == owner_id)
        {
            Some(job) => {
                let pid = job.pid();
                let stop_type = if gracefully {
                    StopType::Stop
                } else {
                    StopType::Kill
                };
                match std::process::Command::new("kill")
                    .arg(stop_type.flag())
                    .arg(pid.to_string())
                    .spawn()
                {
                    Ok(child) => match child.wait_with_output() {
                        Ok(_) => {
                            *job.status().lock().unwrap() = Status::Stopped(stop_type);
                            Ok(())
                        }
                        Err(e) => Err(Error::JobStopErr(format!(
                            "failed to {} job {}: {:?}",
                            stop_type.as_str(),
                            job_id,
                            e
                        ))),
                    },
                    Err(e) => Err(Error::JobStopErr(format!(
                        "failed to execute {} command: {:?}",
                        stop_type.as_str(),
                        e
                    ))),
                }
            }
            None => Err(Error::JobStopErr(format!(
                "no job found with id {}",
                job_id
            ))),
        }
    }

    pub fn query(&self, _job_id: Uuid) -> Result<Status, Error> {
        todo!()
    }

    pub fn stream(
        &self,
        _ctx: std::task::Context,
        _process_id: Uuid,
    ) -> Result<std::io::BufReader<File>, Error> {
        todo!()
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
    *status.lock().unwrap() = Status::Exited(output.status.code());
}
