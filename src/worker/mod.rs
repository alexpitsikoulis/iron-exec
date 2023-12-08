mod cgroup;
mod job;
mod tests;
use cgroup::{add_to_cgroup, create_cgroup};
use job::{Command, Job, Status};
use nix::{
    sys::wait::waitpid,
    unistd::{execv, fork, getpid, ForkResult},
};
use std::{
    ffi::CString,
    fs::File,
    process::Stdio,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use self::{cgroup::CgroupConfig, job::ProcessState};

#[derive(Clone, Debug)]
pub enum Error {
    JobStartErr(String),
    JobStopErr(String),
    JobQueryErr(String),
    JobStreamErr(String),
}

#[derive(Clone, Debug)]
pub struct WorkerJob {
    id: Uuid,
    status: Arc<Mutex<Status>>,
    owner_id: Uuid,
}

#[derive(Clone)]
pub struct Worker {
    pub jobs: Vec<Box<WorkerJob>>,
}

impl Worker {
    pub fn new() -> Self {
        Worker { jobs: vec![] }
    }

    pub fn start(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
    ) -> (Uuid, tokio::task::JoinHandle<Result<(), Error>>) {
        let job_id = Uuid::new_v4();
        let status = Arc::new(Mutex::new(Status::new(0, 0, ProcessState::UnknownState)));
        let mut cmd = std::process::Command::new(command.name());
        let cmd = cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child_proc = Arc::new(Mutex::new(cmd.args(command.args()).spawn().unwrap()));
        let (stdout_handle, stderr_handle) = Job::start_logger(
            format!("{}_{}.log", command.name(), job_id),
            child_proc.clone(),
        );
        let mut job = Job::new(
            job_id,
            command,
            status.clone(),
            Uuid::new_v4(),
            stdout_handle,
            stderr_handle,
        );
        self.jobs.push(Box::new(WorkerJob {
            id: job.id(),
            status: job.status(),
            owner_id: job.owner_id(),
        }));
        let job_thread = tokio::spawn(async move {
            job.update_state(ProcessState::Running);
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
                        job.wait(status, child_proc);
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
                                    job.wait(status, child_proc);
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
                    job.wait(status, child_proc);
                    Ok(())
                }
            }
        });
        (job_id, job_thread)
    }

    pub fn query(&self, job_id: Uuid) -> Result<Status, Error> {
        todo!()
    }

    pub fn stream(
        &self,
        _ctx: std::task::Context,
        _process_id: Uuid,
    ) -> Result<std::io::BufReader<File>, Error> {
        todo!()
    }
}
