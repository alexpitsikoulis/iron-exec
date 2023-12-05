mod cgroup;
mod job;
mod pipe_logger;
mod tests;
use async_trait::async_trait;
use cgroup::{add_to_cgroup, create_cgroup};
use job::{Command, Job, Status};
use nix::{
    sys::wait::waitpid,
    unistd::{execv, fork, getpid, pipe, ForkResult},
};
use pipe_logger::PipeLogger;
use std::{
    borrow::BorrowMut,
    ffi::CString,
    fs::File,
    io::prelude::*,
    ops::DerefMut,
    os::fd::{FromRawFd, IntoRawFd},
    process::Stdio,
    sync::Arc,
};
use uuid::Uuid;

use self::{cgroup::CgroupConfig, job::ProcessState};

#[derive(Debug)]
pub enum Error {
    JobStartErr(String),
    JobStopErr(String),
    JobQueryErr(String),
    JobStreamErr(String),
}

pub struct Worker {
    jobs: Vec<Job>,
}

impl Worker {
    pub fn new() -> Self {
        Worker { jobs: vec![] }
    }

    async fn start(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
    ) -> Result<Job, Error> {
        let job_id = Uuid::new_v4();
        let mut cmd = std::process::Command::new(command.name());
        let cmd = cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        match cmd.args(command.args()).spawn() {
            Ok(child) => {
                let mut pid = child.id();
                let mut job = Job::new(
                    job_id,
                    command,
                    child,
                    ProcessState::UnknownState,
                    Uuid::new_v4(),
                );

                match cgroup_config {
                    Some(config) => match unsafe { fork() } {
                        Ok(ForkResult::Parent { child, .. }) => {
                            if let Err(e) = waitpid(child, None) {
                                return Err(Error::JobStartErr(format!("waitpid failed: {:?}", e)));
                            }
                            job.status_mut().set_pid(child.as_raw() as u32);
                            Ok(job)
                        }
                        Ok(ForkResult::Child) => {
                            match create_cgroup(command.name(), job_id, config) {
                                Ok(cgroup_path) => {
                                    pid = getpid().as_raw() as u32;
                                    if let Err(e) = add_to_cgroup(pid, cgroup_path) {
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
                                            status.set_pid(pid);
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
                        status.set_pid(pid);
                        Ok(job)
                    }
                }
            }
            Err(e) => Err(Error::JobStartErr(e.to_string())),
        }
    }

    fn stop(&mut self, job_id: Uuid) -> Result<(), Error> {
        let mut job_index = -1;
        self.jobs.clone().iter().enumerate().find(|(i, job)| {
            if job.id() == job_id {
                job_index = *i as i32;
                job.proc_lock()
                    .expect("failed to lock worker's jobs list mutex")
                    .kill()
                    .map_err(|e| {
                        Error::JobStopErr(format!("failed to kill job '{}': {:?}", job_id, e))
                    })
                    .unwrap();
                true
            } else {
                false
            }
        });
        if job_index > -1 {
            self.jobs.remove(job_index as usize);
            Ok(())
        } else {
            Err(Error::JobStopErr(format!(
                "no job found with id '{}'",
                job_id
            )))
        }
    }

    fn query(&self, job_id: Uuid) -> Result<Status, Error> {
        todo!()
    }

    fn stream(
        &self,
        _ctx: std::task::Context,
        _process_id: Uuid,
    ) -> Result<std::io::BufReader<File>, Error> {
        todo!()
    }
}
