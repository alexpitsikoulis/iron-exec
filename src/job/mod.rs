mod cgroup;
mod command;
mod status;
pub use cgroup::*;
pub use command::*;
pub use status::*;

use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Job {
    id: Uuid,
    cmd: Command,
    pid: u32,
    status: Arc<Mutex<Status>>,
    owner_id: Uuid,
}

impl Job {
    pub fn new(
        id: Uuid,
        cmd: Command,
        pid: u32,
        status: Arc<Mutex<Status>>,
        owner_id: Uuid,
    ) -> Self {
        Job {
            id,
            cmd,
            pid,
            status,
            owner_id,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn cmd(&self) -> Command {
        self.cmd
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
}
