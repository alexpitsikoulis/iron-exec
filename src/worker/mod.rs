use crate::{
    config::Config,
    job::{add_to_cgroup, create_cgroup, CgroupConfig, Command, Job, ProcessState, Status},
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
    ) -> (Uuid, tokio::task::JoinHandle<Result<(), Error>>) {
        let job_id = Uuid::new_v4();
        let mut cmd = std::process::Command::new(command.name());
        let mut cmd = cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
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

        let status = Arc::new(Mutex::new(Status::new(
            child_proc.lock().unwrap().id(),
            0,
            ProcessState::UnknownState,
        )));
        let mut job = Box::new(Job::new(job_id, command, status.clone(), Uuid::new_v4()));
        self.jobs.push(job.clone());
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
        (job_id, job_thread)
    }

    pub fn stop(&mut self, _job_id: Uuid) -> Result<(), Error> {
        todo!();
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
    let mut status = status.lock().unwrap();
    let proc_lock = Arc::try_unwrap(proc).unwrap();
    let inner = proc_lock.into_inner().unwrap();
    let pid = inner.id();
    status.set_pid(pid);
    let output = inner.wait_with_output().unwrap();
    status.set_state(ProcessState::Exited);
    status.set_exit_code(output.status.code().unwrap());
}
