use std::{env::current_dir, fs};

pub const LOG_DIR: &'static str = "./tests/.logs";

pub struct TestLog {}

impl TestLog {
    pub fn new() -> Self {
        TestLog {}
    }

    #[allow(dead_code)]
    pub fn consume(&self, log_filename: String) -> Vec<u8> {
        let log_file_path = current_dir()
            .expect("failed to determine the current directory")
            .join(LOG_DIR)
            .join(log_filename);
        let content = fs::read(log_file_path.clone()).expect(
            format!(
                "expected log file {} does not exist",
                log_file_path.to_str().unwrap()
            )
            .as_str(),
        );
        fs::remove_file(log_file_path).unwrap();
        content
    }
}
