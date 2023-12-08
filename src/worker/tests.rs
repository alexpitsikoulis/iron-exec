#[cfg(test)]
mod tests {
    use crate::worker::{
        job::{Command, ProcessState},
        Worker,
    };
    use claim::assert_ok;

    #[tokio::test]
    pub async fn test_echo_hello_world() {
        let mut worker = Worker::new();
        let (_, job_handle) = worker.start(Command::new("echo", &["hello", "world"]), None);
        assert_eq!(
            1,
            worker.jobs.len(),
            "job was not pushed to worker jobs vec"
        );
        assert_eq!(
            ProcessState::UnknownState,
            worker.jobs.get(0).unwrap().status().lock().unwrap().state(),
            "job was not in expected state"
        );
        assert_ne!(
            0,
            worker.jobs.get(0).unwrap().status().lock().unwrap().pid(),
            "pid was not assigned to job"
        );
        let res = job_handle.await.unwrap();
        assert_ok!(res.clone(), "failed to join job thread: {:?}", res,);
        assert_eq!(
            ProcessState::Exited,
            worker.jobs.get(0).unwrap().status().lock().unwrap().state(),
            "job was not exited at end of execution"
        );
        assert_eq!(0, worker.jobs.get(0).unwrap().status().lock().unwrap().exit_code(), "job exited with non-zero exit code");
    }
}
