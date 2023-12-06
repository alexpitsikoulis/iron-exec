#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use claim::assert_ok;
    use log::info;
    use uuid::Uuid;

    use crate::worker::{
        job::{Command, ProcessState},
        Worker,
    };

    #[tokio::test]
    pub async fn test_echo_hello_world() {
        // let mut worker = Worker::new();
        // let mut job = worker
        //     .start(Command::new("echo", &["hello", "world"]), None)
        //     .await
        //     .unwrap();
        // let log_file_path = format!("./echo_{}.log", job.id());

        // assert_ok!(
        //     std::fs::metadata(log_file_path.clone()),
        //     "log file not found",
        // );
        // let log_file_content = std::fs::read_to_string(log_file_path).unwrap();
        // assert_eq!(
        //     "stdout: hello world\n", log_file_content,
        //     "log file did not have expected content",
        // );
        let mut worker = Worker::new();
        let (job, job_handle) = worker.start(Command::new("echo", &["hello", "world"]), None);
        let res = job_handle.await.unwrap();
        assert_ok!(res.clone(), "failed to join job thread: {:?}", res,);

        let job_lock = job.lock().unwrap();
        let status = job_lock.status();

        assert_eq!(
            0,
            status.exit_code(),
            "process exited with non-zero status code",
        );
        assert_eq!(
            ProcessState::Exited,
            status.state(),
            "process was not exited",
        );
    }

    // #[tokio::test]
    // pub async fn test_stop_job() {
    //     let mut job_id: Option<Uuid> = None;
    //     let mut worker = Worker::new();
    //     let mut worker_clone = worker.clone();
    //     let start_thread = tokio::spawn(async move {
    //         let mut job = worker
    //             .start(Command::new("sh", &["./test_scripts/infinite_loop.sh"]), None)
    //             .await
    //             .unwrap();
    //     });
    //     let stop_thread = tokio::spawn(async move {

    //     })

    // }
}
