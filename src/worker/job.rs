use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    process::Child,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Job {
    id: Uuid,
    cmd: Command,
    status: Arc<Mutex<Status>>,
    owner_id: Uuid,
}

impl Job {
    pub fn new(id: Uuid, cmd: Command, status: Arc<Mutex<Status>>, owner_id: Uuid) -> Self {
        Job {
            id,
            cmd,
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

    pub fn status(&self) -> Arc<Mutex<Status>> {
        self.status.clone()
    }

    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    pub fn update_state(&mut self, state: ProcessState) {
        let mut status = self.status.lock().unwrap();
        status.state = state;
    }

    pub fn start_logger(
        log_filename: impl ToString,
        proc: Arc<Mutex<Child>>,
    ) -> (JoinHandle<()>, JoinHandle<()>) {
        let mut proc = proc.lock().unwrap();
        let stdout = BufReader::new(proc.stdout.take().unwrap());
        let stderr = BufReader::new(proc.stderr.take().unwrap());
        drop(proc);
        let log_file = Arc::new(Mutex::new(
            File::create(format!("./{}", log_filename.to_string())).unwrap(),
        ));

        let file_clone = log_file.clone();
        let stdout_handle = thread::spawn(move || {
            for line in stdout.lines() {
                let line = line.unwrap();
                let mut file = file_clone.lock().unwrap();
                writeln!(file, "stdout: {}", line).unwrap();
            }
        });

        let stderr_handle = thread::spawn(move || {
            for line in stderr.lines() {
                let line = line.unwrap();
                let mut file = log_file.lock().unwrap();
                writeln!(file, "stderr: {}", line).unwrap();
            }
        });

        (stdout_handle, stderr_handle)
    }

    pub fn close_logger(logger: (JoinHandle<()>, JoinHandle<()>)) {
        logger.0.join().unwrap();
        logger.1.join().unwrap();
    }

    pub fn wait(
        self,
        status: Arc<Mutex<Status>>,
        proc: Arc<Mutex<Child>>,
        logger: (JoinHandle<()>, JoinHandle<()>),
    ) {
        Self::close_logger(logger);
        let mut status = status.lock().unwrap();
        let proc_lock = Arc::try_unwrap(proc).unwrap();
        let inner = proc_lock.into_inner().unwrap();
        let pid = inner.id();
        status.set_pid(pid);
        let output = inner.wait_with_output().unwrap();
        status.set_state(ProcessState::Exited);
        status.set_exit_code(output.status.code().unwrap());
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
