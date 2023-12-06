use std::{
    ops::DerefMut,
    process::Child,
    sync::{Arc, Mutex}, thread::JoinHandle,
};
use uuid::Uuid;

use super::pipe_logger::PipeLogger;

#[derive(Debug)]
pub struct Job {
    id: Uuid,
    cmd: Command,
    proc: Option<Arc<Mutex<Child>>>,
    status: Status,
    owner_id: Uuid,
    logger: Option<PipeLogger>,
}

impl Job {
    pub fn new(id: Uuid, cmd: Command, state: ProcessState, owner_id: Uuid) -> Self {
        Job {
            id,
            cmd,
            proc: None,
            status: Status {
                pid: 0,
                exit_code: 0,
                state,
            },
            owner_id,
            logger: None,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn proc(&self) -> &Arc<Mutex<Child>> {
        self.proc.as_ref().expect("child process has detached from job")
    }

    pub fn proc_lock(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, Child>,
        std::sync::PoisonError<std::sync::MutexGuard<'_, Child>>,
    > {
        self.proc().lock()
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn status_mut(&mut self) -> &mut Status {
        &mut self.status
    }

    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    pub fn set_proc(&mut self, proc: Arc<Mutex<Child>>) -> (JoinHandle<()>, JoinHandle<()>) {
        let proc_clone = proc.clone();
        let mut proc_lock = proc_clone.lock().unwrap();
        let logger = PipeLogger::new(
            format!("{}_{}.log", self.cmd.name(), self.id),
            &mut proc_lock,
        );
        drop(proc_lock);
        self.proc = Some(proc.clone());
        logger
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Command {
    name: &'static str,
    args: &'static [&'static str],
}

impl Command {
    pub fn new(name: &'static str, args: &'static [&'static str]) -> Self {
        Command { name, args }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn args(&self) -> &'static [&'static str] {
        self.args
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessState {
    UnknownState,
    Running,
    Exited,
}

#[derive(Clone, Debug)]
pub struct Status {
    pid: u32,
    exit_code: i32,
    state: ProcessState,
}

impl Status {
    pub fn new(pid: u32, exit_code: i32, state: ProcessState) -> Self {
        Status {
            pid,
            exit_code,
            state,
        }
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn set_pid(&mut self, pid: u32) {
        self.pid = pid;
    }

    pub fn set_exit_code(&mut self, exit_code: i32) {
        self.exit_code = exit_code;
    }

    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }
}
