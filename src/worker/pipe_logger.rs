use std::{
    any::Any,
    fs::File,
    future::{self, Future},
    io::{BufRead, BufReader, Take, Write},
    ops::Deref,
    os::fd::{FromRawFd, IntoRawFd},
    process::{Child, ChildStderr, ChildStdout, Stdio},
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread,
};

use std::thread::JoinHandle;

#[derive(Debug)]
pub struct PipeLogger {}

impl PipeLogger {
    pub fn new(log_filename: impl ToString, proc: &mut Child) -> (JoinHandle<()>, JoinHandle<()>) {
        let stdout = BufReader::new(proc.stdout.take().unwrap());
        let stderr = BufReader::new(proc.stderr.take().unwrap());
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

        return(stdout_handle, stderr_handle)
    }
    
    pub async fn close(self) {
        
    }
}
