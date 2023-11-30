use uuid::Uuid;

pub struct Job {
    id: Uuid,
    cmd: std::process::Command,
    status: Status,
    owner_id: Uuid,
}

impl Job {
    pub fn new(id: Uuid, cmd: std::process::Command, state: ProcessState, owner_id: Uuid) -> Self {
        Job {
            id,
            cmd,
            status: Status {
                pid: -1,
                exit_code: 0,
                state,
            },
            owner_id,
        }
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
pub struct Command {
    name: &'static str,
    args: &'static [&'static str],
}

impl Command {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn args(&self) -> &'static [&'static str] {
        self.args
    }
}

pub enum ProcessState {
    UnknownState,
    Running,
    Exited,
}

pub struct Status {
    pid: i32,
    exit_code: i8,
    state: ProcessState,
}

impl Status {
    pub fn set_pid(&mut self, pid: i32) {
        self.pid = pid;
    }

    pub fn set_exit_code(&mut self, exit_code: i8) {
        self.exit_code = exit_code;
    }

    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }
}
