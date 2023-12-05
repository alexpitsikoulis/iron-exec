#[cfg(test)]
mod tests {
    use claim::assert_ok;
    use log::info;

    use crate::worker::{job::Command, Worker};

    #[tokio::test]
    pub async fn test_echo_hello_world() {
        let mut worker = Worker::new();
        let mut job = worker
            .start(Command::new("echo", &["hello", "world"]), None)
            .await
            .unwrap();
        let log_file_path = format!("./echo_{}.log", job.id());

        assert_ok!(
            std::fs::metadata(log_file_path.clone()),
            "log file not found",
        );
        let log_file_content = std::fs::read_to_string(log_file_path).unwrap();
        assert_eq!(
            "stdout: hello world\n", log_file_content,
            "log file did not have expected content",
        );
    }
}
