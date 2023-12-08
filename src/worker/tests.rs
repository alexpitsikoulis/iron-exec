#[cfg(test)]
mod tests {
    use crate::worker::{job::Command, Worker};
    use claim::assert_ok;

    #[tokio::test]
    pub async fn test_echo_hello_world() {
        let mut worker = Worker::new();
        println!("JOBS: {:?}", worker.jobs);
        let (_, job_handle) = worker.start(Command::new("echo", &["hello", "world"]), None);
        println!("JOBS: {:?}", worker.jobs);
        println!("WAITING");
        let res = job_handle.await.unwrap();
        println!("JOBS: {:?}", worker.jobs);
        assert_ok!(res.clone(), "failed to join job thread: {:?}", res,);
    }
}
