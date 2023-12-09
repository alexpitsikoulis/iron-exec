mod utils;

use std::{thread, time::Duration};

use claim::assert_ok;
use iron_exec::job::{Command, Status, StopType};
use utils::{logs, worker::TestWorker};
use uuid::Uuid;

#[tokio::test]
pub async fn test_kill_infinite_loop() {
    let mut worker = TestWorker::new();

    let (job, wait_thread) = worker.queue_job(
        Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
        None,
    );

    thread::sleep(Duration::from_secs(1));

    assert_ok!(worker.0.stop(job.id(), job.owner_id(), false));

    assert_eq!(
        Status::Stopped(StopType::Kill),
        *worker.0.jobs[0].status().lock().unwrap(),
        "job did not exit with killed status",
    );

    assert_ok!(wait_thread.join());
}

#[tokio::test]
pub async fn test_stop_infinite_loop() {
    let mut worker = TestWorker::new();
    let (job, job_handle) = worker.0.start(
        Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
        None,
        Uuid::new_v4(),
    );

    let wait_thread = thread::spawn(move || {
        job_handle.join().unwrap().unwrap();
    });

    thread::sleep(Duration::from_secs(1));

    assert_ok!(worker.0.stop(job.id(), job.owner_id(), true));

    assert_eq!(
        Status::Stopped(StopType::Stop),
        *worker.0.jobs[0].status().lock().unwrap(),
        "job did not exit with stopped status",
    );
    logs::consume(format!("sh_{}.log", job.id()));

    let proc_file =
        std::fs::read_to_string(format!("/proc/{}/status", worker.0.jobs[0].pid())).unwrap();
    assert!(
        proc_file.contains("stopped"),
        "process file does not reflect stopped state",
    );

    worker.0.stop(job.id(), job.owner_id(), false).unwrap();

    assert_ok!(wait_thread.join());
}
