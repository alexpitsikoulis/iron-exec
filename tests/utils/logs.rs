use std::{env::current_dir, fs, path::PathBuf};

pub const LOG_DIR: &'static str = "./tests/logs";

pub fn read(log_filename: String) -> Vec<u8> {
    let log_file_path = get_log_dir_path().join(log_filename);
    fs::read(log_file_path.clone()).expect(format!("expected log file {} does not exist", log_file_path.to_str().unwrap()).as_str())
}

pub fn clear() {
    let log_dir_path = get_log_dir_path().join(LOG_DIR);
}

fn get_log_dir_path() -> PathBuf {
    current_dir()
        .expect("failed to determine the current directory")
        .join(LOG_DIR)
}
