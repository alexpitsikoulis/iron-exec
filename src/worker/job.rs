use std::{
    ops::DerefMut,
    process::Child,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use super::pipe_logger::PipeLogger;

#[derive(Clone)]
pub struct Job {
    id: Uuid,
    cmd: Command,
    proc: Arc<Mutex<Child>>,
    status: Status,
    owner_id: Uuid,
}

impl Job {
    pub fn new(
        id: Uuid,
        cmd: Command,
        mut proc: Child,
        state: ProcessState,
        owner_id: Uuid,
    ) -> Self {
        PipeLogger::start(format!("{}_{}.log", cmd.name(), id), &mut proc);
        Job {
            id,
            cmd,
            proc: Arc::new(Mutex::new(proc)),
            status: Status {
                pid: 0,
                exit_code: 0,
                state,
            },
            owner_id,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn proc(&self) -> Arc<Mutex<Child>> {
        self.proc.clone()
    }

    pub fn proc_lock(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, Child>,
        std::sync::PoisonError<std::sync::MutexGuard<'_, Child>>,
    > {
        self.proc.lock()
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
}

#[derive(Clone, Copy)]
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

#[derive(Clone)]
pub enum ProcessState {
    UnknownState,
    Running,
    Exited,
}

#[derive(Clone)]
pub struct Status {
    pid: u32,
    exit_code: i32,
    state: ProcessState,
}

impl Status {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
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
