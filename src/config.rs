#[derive(Clone)]
pub struct Config {
    pub log_dir: &'static str,
}

impl Config {
    pub fn new(log_dir: &'static str) -> Self {
        Config { log_dir }
    }
}
