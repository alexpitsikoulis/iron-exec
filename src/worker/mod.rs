mod cgroup;
mod job;
use cgroup::{add_to_cgroup, create_cgroup};
use job::{Command, Job, Status};
use log::Log;
use nix::{
    sys::wait::waitpid,
    unistd::{execv, fork, getpid, ForkResult},
};
use std::{ffi::CString, fs::File, sync::Arc};
use uuid::Uuid;

use self::{cgroup::CgroupConfig, job::ProcessState};

pub enum Error {
    JobStartErr(String),
    JobStopErr(String),
    JobQueryErr(String),
    JobStreamErr(String),
}

pub trait ProcessRunner {
    fn start(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
    ) -> Result<Job, Error>;
    fn stop(&mut self, job_id: &'static str) -> Result<(), Error>;
    fn query(&self, job_id: &'static str) -> Result<Status, Error>;
    fn stream(
        &self,
        ctx: std::task::Context,
        process_id: &'static str,
    ) -> Result<std::io::BufReader<File>, Error>;
}

pub struct Worker {
    _logger: &'static dyn Log,
    jobs: Vec<Arc<Job>>,
}

impl Worker {
    pub fn new(logger: &'static dyn Log) -> Self {
        Worker {
            _logger: logger,
            jobs: Vec::new(),
        }
    }

    pub fn add_job(&mut self, job: Job) {
        self.jobs.push(Arc::new(job));
    }
}

impl ProcessRunner for Worker {
    fn start(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
    ) -> Result<Job, Error> {
        let mut base_command = std::process::Command::new(command.name());
        let with_args = base_command.args(command.args());
        match with_args.spawn() {
            Ok(child) => {
                let job_id = Uuid::new_v4();
                let mut job = Job::new(
                    job_id,
                    base_command,
                    ProcessState::UnknownState,
                    Uuid::new_v4(),
                );
                match cgroup_config {
                    Some(config) => match unsafe { fork() } {
                        Ok(ForkResult::Parent { child, .. }) => {
                            if let Err(e) = waitpid(child, None) {
                                return Err(Error::JobStartErr(format!("waitpid failed: {:?}", e)));
                            }
                            job.status_mut().set_pid(child.as_raw());
                            Ok(job)
                        }
                        Ok(ForkResult::Child) => {
                            match create_cgroup(command.name(), job_id, config) {
                                Ok(cgroup_path) => {
                                    let pid = getpid();
                                    if let Err(e) = add_to_cgroup(pid.as_raw(), cgroup_path) {
                                        return Err(Error::JobStartErr(e.to_string()));
                                    }

                                    let cmd = CString::new(command.name()).unwrap();
                                    let mut args: Vec<CString> = Vec::new();
                                    for arg in command.args() {
                                        args.push(CString::new(*arg).unwrap())
                                    }
                                    match execv(&cmd, &args) {
                                        Ok(_) => {
                                            let status = job.status_mut();
                                            status.set_pid(pid.as_raw());
                                            Ok(job)
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
                            }
                        }
                        Err(e) => Err(Error::JobStartErr(format!(
                            "failed to fork process: {:?}",
                            e
                        ))),
                    },
                    None => {
                        let status = job.status_mut();
                        status.set_pid(child.id() as i32);
                        Ok(job)
                    }
                }
            }
            Err(e) => Err(Error::JobStartErr(e.to_string())),
        }
    }

    fn stop(&mut self, _job_id: &'static str) -> Result<(), Error> {
        todo!()
    }

    fn query(&self, _job_id: &'static str) -> Result<Status, Error> {
        todo!()
    }

    fn stream(
        &self,
        _ctx: std::task::Context,
        _process_id: &'static str,
    ) -> Result<std::io::BufReader<File>, Error> {
        todo!()
    }
}
