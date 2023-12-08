use std::{sync::{Arc, Mutex}, process::Child, io::{BufReader, BufRead, Write}, fs::File, thread::{self, JoinHandle}};

pub struct PipeLogger {
    stdout: JoinHandle<()>,
    stderr: JoinHandle<()>,
}

impl PipeLogger {
    pub fn new(log_filename: impl ToString, proc: Arc<Mutex<Child>>) -> Self {
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

        PipeLogger { stdout: stdout_handle, stderr: stderr_handle }
    }

    pub fn close(self) {
        self.stdout.join().unwrap();
        self.stderr.join().unwrap();
    }
}