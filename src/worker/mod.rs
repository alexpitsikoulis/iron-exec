mod cgroup;
mod job;
mod pipe_logger;
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
    process::{Child, Stdio},
    sync::{Arc, Mutex},
    thread::JoinHandle,
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

#[derive(Clone)]
pub struct Worker {
    jobs: Vec<Arc<Mutex<Job>>>,
}

impl Worker {
    pub fn new() -> Self {
        Worker { jobs: vec![] }
    }

    pub fn start(
        &mut self,
        command: Command,
        cgroup_config: Option<CgroupConfig>,
    ) -> (Arc<Mutex<Job>>, tokio::task::JoinHandle<Result<(), Error>>) {
        let job_id = Uuid::new_v4();
        let job = Arc::new(Mutex::new(Job::new(
            job_id,
            command,
            ProcessState::UnknownState,
            Uuid::new_v4(),
        )));
        let thread_job = job.clone();
        let job_thread = tokio::spawn(async move {
            let mut cmd = std::process::Command::new(command.name());
            let cmd = cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            match cmd.args(command.args()).spawn() {
                Ok(child_proc) => {
                    let job = thread_job.clone();
                    let mut job = job.lock().unwrap();
                    let child_mtx = Arc::new(Mutex::new(child_proc));
                    let logger = job.set_proc(child_mtx.clone());
                    job.status_mut().set_state(ProcessState::Running);

                    match cgroup_config {
                        Some(config) => match unsafe { fork() } {
                            Ok(ForkResult::Parent { child, .. }) => {
                                if let Err(e) = waitpid(child, None) {
                                    job.proc_lock()
                                        .unwrap()
                                        .kill()
                                        .expect("failed to kill process after waitpid failed");
                                    panic!("waitpid failed: {:?}", e);
                                }
                                drop(job);
                                Self::wait(thread_job, child_mtx, logger);
                                Ok(())
                            }
                            Ok(ForkResult::Child) => {
                                match create_cgroup(command.name(), job_id, config) {
                                    Ok(cgroup_path) => {
                                        let pid = getpid().as_raw() as u32;
                                        if let Err(e) = add_to_cgroup(pid, cgroup_path) {
                                            job.proc_lock().unwrap().kill().expect("failed to kill process after it failed to be added to the cgroup");
                                            panic!("add_to_cgroup failed: {:?}", e);
                                        }

                                        let cmd = CString::new(command.name()).unwrap();
                                        let mut args: Vec<CString> = Vec::new();
                                        for arg in command.args() {
                                            args.push(CString::new(*arg).unwrap())
                                        }
                                        match execv(&cmd, &args) {
                                            Ok(_) => {
                                                Self::wait(thread_job, child_mtx, logger);
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
                                }
                            }
                            Err(e) => Err(Error::JobStartErr(format!(
                                "failed to fork process: {:?}",
                                e
                            ))),
                        },
                        None => {
                            drop(job);
                            println!("MADE IT TO WAIT");
                            Self::wait(thread_job, child_mtx, logger);
                            println!("FINISHED WAIT");
                            Ok(())
                        }
                    }
                }
                Err(e) => Err(Error::JobStartErr(e.to_string())),
            }
        });
        (job, job_thread)
    }

    pub fn stop(&mut self, job_id: Uuid) -> Result<(), Error> {
        match self.jobs.iter().find(|job| {
            let job = job.lock().unwrap();
            job.id() == job_id
        }) {
            Some(job) => {
                let job_lock = job.lock().unwrap();
                let mut proc = job_lock.proc_lock().map_err(|e| {
                    Error::JobStopErr(format!("failed to initiate mutex lock: {:?}", e))
                })?;
                match proc.kill() {
                    Ok(()) => {
                        drop(proc);
                        drop(job_lock);
                        job.lock().unwrap().status_mut().set_exit_code(137);
                        Ok(())
                    }
                    Err(e) => Err(Error::JobStopErr(format!(
                        "failed to kill process {}: {:?}",
                        proc.id(),
                        e
                    ))),
                }
            }
            None => Err(Error::JobStopErr(format!(
                "no job found with id '{}'",
                job_id
            ))),
        }
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

    fn wait(job: Arc<Mutex<Job>>, proc: Arc<Mutex<Child>>, logger: (JoinHandle<()>, JoinHandle<()>)) {
        logger.0.join().unwrap();
        logger.1.join().unwrap();
        println!("MADE IT INTO WAIT");
        let mut job = job.lock().unwrap();
        println!("MADE IT PAST JOB LOCK");
        let lock = Arc::try_unwrap(proc);
        while !lock.is_ok() {}
        let lock = lock.unwrap();
        let inner = lock.into_inner().unwrap();
        let pid = inner.id();
        let out = inner.wait_with_output().unwrap();
        let status = job.status_mut();
        status.set_pid(pid);
        status.set_state(ProcessState::Exited);
        status.set_exit_code(out.status.code().unwrap());
    }
}
