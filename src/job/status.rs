#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessState {
    UnknownState,
    Running,
    Exited,
}

#[derive(Clone, Copy, Debug)]
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
