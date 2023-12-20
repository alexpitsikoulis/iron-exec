#[derive(Debug, Clone)]
pub struct Config {
    log_dir: &'static str,
    thread_count: usize,
}

impl Config {
    pub fn new(log_dir: &'static str, thread_count: usize) -> Self {
        Config {
            log_dir,
            thread_count,
        }
    }

    pub fn default() -> Self {
        Config {
            log_dir: "/tmp",
            thread_count: 4,
        }
    }

    pub fn log_dir(&self) -> &'static str {
        self.log_dir
    }

    pub fn thread_count(&self) -> usize {
        self.thread_count
    }
}
