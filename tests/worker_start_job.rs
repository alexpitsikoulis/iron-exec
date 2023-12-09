mod utils;

use claim::assert_ok;
use iron_exec::job::{Command, ProcessState};
use utils::{logs, worker::TestWorker};

#[tokio::test]
pub async fn test_echo_hello_world() {
    let mut worker = TestWorker::new();
    let (job_id, job_handle) = worker
        .0
        .start(Command::new("echo", &["hello", "world"]), None);
    assert_eq!(
        1,
        worker.0.jobs.len(),
        "job was not pushed to worker jobs vec"
    );
    assert_eq!(
        ProcessState::UnknownState,
        worker
            .0
            .jobs
            .get(0)
            .unwrap()
            .status()
            .lock()
            .unwrap()
            .state(),
        "job was not in expected state"
    );
    assert_ne!(
        0,
        worker.0.jobs.get(0).unwrap().status().lock().unwrap().pid(),
        "pid was not assigned to job"
    );
    let res = job_handle.await.unwrap();
    assert_ok!(res.clone(), "failed to join job thread: {:?}", res,);
    assert_eq!(
        ProcessState::Exited,
        worker
            .0
            .jobs
            .get(0)
            .unwrap()
            .status()
            .lock()
            .unwrap()
            .state(),
        "job was not exited at end of execution"
    );
    assert_eq!(
        0,
        worker
            .0
            .jobs
            .get(0)
            .unwrap()
            .status()
            .lock()
            .unwrap()
            .exit_code(),
        "job exited with non-zero exit code"
    );

    let logs = logs::consume(format!("echo_{}.log", job_id));
    assert_eq!(
        "hello world\n".as_bytes(),
        logs,
        "job logs did not match expected content",
    );
}

#[tokio::test]
pub async fn test_both_stdout_and_stderr() {
    let mut worker = TestWorker::new();
    let (job_id, job_handle) = worker.0.start(
        Command::new("sh", &["./tests/scripts/echo_and_error.sh"]),
        None,
    );
    assert_eq!(
        1,
        worker.0.jobs.len(),
        "job was not pushed to worker jobs vec"
    );
    assert_eq!(
        ProcessState::UnknownState,
        worker
            .0
            .jobs
            .get(0)
            .unwrap()
            .status()
            .lock()
            .unwrap()
            .state(),
        "job was not in expected state"
    );
    assert_ne!(
        0,
        worker.0.jobs.get(0).unwrap().status().lock().unwrap().pid(),
        "pid was not assigned to job"
    );
    let res = job_handle.await.unwrap();
    assert_ok!(res.clone(), "failed to join job thread: {:?}", res);
    assert_eq!(
        ProcessState::Exited,
        worker
            .0
            .jobs
            .get(0)
            .unwrap()
            .status()
            .lock()
            .unwrap()
            .state(),
        "job was not exited at end of execution"
    );
    assert_eq!(
        127,
        worker
            .0
            .jobs
            .get(0)
            .unwrap()
            .status()
            .lock()
            .unwrap()
            .exit_code(),
        "job did not exit with expected exit code"
    );

    let logs = logs::consume(format!("sh_{}.log", job_id));
    assert_eq!(
        "testing\none more\nstderr test\nback to stdout\n./tests/scripts/echo_and_error.sh: 5: SET: not found\n".as_bytes(),
        logs.as_slice(),
        "log content different than expected"
    );
}
