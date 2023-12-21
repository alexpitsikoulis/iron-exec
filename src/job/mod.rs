mod command;
mod status;
pub use command::*;
use crossbeam::channel::Sender;
pub use status::*;
use syscalls::{syscall, Sysno};

use std::{
    fs::File,
    io::BufReader,
    os::fd::FromRawFd,
    process::Stdio,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Debug)]
pub struct JobInfo {
    status: String,
    pid: u32,
    exit_code: Option<i32>,
    command: Command,
}

impl JobInfo {
    pub fn status(&self) -> String {
        self.status.clone()
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    pub fn command(&self) -> Command {
        self.command.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    id: Uuid,
    command: Command,
    pid: u32,
    status: Arc<Mutex<Status>>,
    owner_id: Uuid,
}

impl Job {
    pub fn new(
        id: Uuid,
        command: Command,
        pid: u32,
        status: Arc<Mutex<Status>>,
        owner_id: Uuid,
    ) -> Self {
        Job {
            id,
            command,
            pid,
            status,
            owner_id,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn command(&self) -> Command {
        self.command.clone()
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn status(&self) -> Arc<Mutex<Status>> {
        self.status.clone()
    }

    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    pub fn set_pid(&mut self, pid: u32) {
        self.pid = pid;
    }

    pub fn start(
        job_id: Uuid,
        command: Command,
        owner_id: Uuid,
        log_file_fd: i32,
    ) -> Result<(Self, std::process::Child), crate::worker::Error> {
        let mut cmd: &mut std::process::Command = &mut std::process::Command::new(command.name());
        unsafe {
            cmd = cmd
                .stdout(Stdio::from_raw_fd(log_file_fd))
                .stderr(Stdio::from_raw_fd(log_file_fd));
        }
        let child_cmd = cmd.args(command.args());
        let child_proc = match child_cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return Err(crate::worker::Error::JobStartErr(format!(
                    "failed to spawn child process: {:?}",
                    e
                )));
            }
        };

        let status = Arc::new(Mutex::new(Status::UnknownState));

        *status.lock().map_err(|e| {
            crate::worker::Error::JobStartErr(format!(
                "failed to lock status mutex for job {}: {:?}",
                job_id, e
            ))
        })? = Status::Running;

        Ok((
            Job::new(job_id, command, child_proc.id(), status.clone(), owner_id),
            child_proc,
        ))
    }

    pub fn stop(
        &self,
        gracefully: bool,
        sender: Sender<Result<(Uuid, bool), crate::worker::Error>>,
    ) -> Result<(), crate::worker::Error> {
        let stop_type = match gracefully {
            true => StopType::Term,
            false => StopType::Kill,
        };
        unsafe {
            match syscall!(Sysno::kill, self.pid, stop_type.sig()) {
                Ok(_) => {
                    *self.status().lock().map_err(|e| {
                        crate::worker::Error::JobStartErr(format!(
                            "failed to lock status mutex for job {}: {:?}",
                            self.id, e
                        ))
                    })? = Status::Stopped(stop_type);
                    if let Err(e) = sender.send(Ok((self.id, true))) {
                        panic!("failed to send stop result: {:?}", e);
                    }
                    Ok(())
                }
                Err(e) => {
                    if let Err(e) = sender.send(Err(crate::worker::Error::JobStopErr(
                        String::from("failed to stop job"),
                    ))) {
                        panic!("failed to send stop error: {:?}", e);
                    };
                    Err(crate::worker::Error::JobStopErr(format!(
                        "failed to send SIG{} to job: {:?}",
                        stop_type.as_str().to_uppercase(),
                        e
                    )))
                }
            }
        }
    }

    pub fn query(&self) -> Result<JobInfo, crate::worker::Error> {
        let status = match self.status.lock() {
            Ok(status) => status,
            Err(e) => {
                return Err(crate::worker::Error::JobQueryErr(format!(
                    "failed to lock status mutex for query: {:?}",
                    e
                )))
            }
        };
        let exit_code = match *status {
            Status::Exited(status_code) => status_code,
            _ => None,
        };
        Ok(JobInfo {
            pid: self.pid,
            command: self.command.clone(),
            status: status.to_string(),
            exit_code,
        })
    }

    pub fn stream(&self, log_dir: &str) -> Result<std::io::BufReader<File>, crate::worker::Error> {
        match std::fs::File::open(format!(
            "{}/{}_{}.log",
            log_dir,
            self.command.name(),
            self.id,
        )) {
            Ok(log_file) => Ok(BufReader::new(log_file)),
            Err(e) => Err(crate::worker::Error::JobStreamErr(format!(
                "failed to open log file: {:?}",
                e
            ))),
        }
    }

    pub fn wait(&self, proc: std::process::Child) -> Result<(Uuid, bool), crate::worker::Error> {
        let output = match proc.wait_with_output() {
            Ok(out) => out,
            Err(e) => {
                return Err(crate::worker::Error::JobErr(
                    self.id,
                    format!("child process failed: {:?}", e),
                ))
            }
        };
        match self.status.lock() {
            Ok(mut status) => {
                if !status.is_stopped() {
                    *status = Status::Exited(output.status.code())
                }
            }
            Err(e) => {
                return Err(crate::worker::Error::JobErr(
                    self.id,
                    format!("failed to lock status mutex to update exit code: {:?}", e),
                ))
            }
        }
        Ok((self.id, false))
    }
}
