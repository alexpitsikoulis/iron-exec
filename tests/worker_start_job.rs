mod utils;

use claim::assert_ok;
use iron_exec::job::{Command, Status};
use utils::app::TestApp;
use uuid::Uuid;

#[tokio::test]
pub async fn test_echo_hello_world() {
    let mut app = TestApp::new();
    let (job, job_handle) = app.worker.start(
        Command::new("echo", &["hello", "world"]),
        None,
        Uuid::new_v4(),
    );
    assert_eq!(
        1,
        app.worker.jobs.len(),
        "job was not pushed to worker jobs vec"
    );
    assert_eq!(
        Status::UnknownState,
        *app.worker.jobs.get(0).unwrap().status().lock().unwrap(),
        "job was not in expected state"
    );
    assert_ne!(
        0,
        app.worker.jobs.get(0).unwrap().pid(),
        "pid was not assigned to job"
    );
    let res = job_handle.join().unwrap();
    assert_ok!(res.clone(), "failed to join job thread: {:?}", res,);
    assert_eq!(
        Status::Exited(Some(0)),
        *app.worker.jobs.get(0).unwrap().status().lock().unwrap(),
        "job did not exit with expected exit code"
    );

    let logs = app.log_handler.consume(format!("echo_{}.log", job.id()));
    assert_eq!(
        "hello world\n".as_bytes(),
        logs,
        "job logs did not match expected content",
    );
}

#[tokio::test]
pub async fn test_both_stdout_and_stderr() {
    let mut app = TestApp::new();
    let (job, job_handle) = app.worker.start(
        Command::new("sh", &["./tests/scripts/echo_and_error.sh"]),
        None,
        Uuid::new_v4(),
    );
    assert_eq!(
        1,
        app.worker.jobs.len(),
        "job was not pushed to worker jobs vec"
    );
    assert_eq!(
        Status::UnknownState,
        *app.worker.jobs.get(0).unwrap().status().lock().unwrap(),
        "job was not in expected state"
    );
    assert_ne!(
        0,
        app.worker.jobs.get(0).unwrap().pid(),
        "pid was not assigned to job"
    );
    let res = job_handle.join().unwrap();
    assert_ok!(res.clone(), "failed to join job thread: {:?}", res);
    assert_eq!(
        Status::Exited(Some(127)),
        *app.worker.jobs.get(0).unwrap().status().lock().unwrap(),
        "job did not exit with expected status code"
    );

    let logs = app.log_handler.consume(format!("sh_{}.log", job.id()));
    assert_eq!(
        "testing\none more\nstderr test\nback to stdout\n./tests/scripts/echo_and_error.sh: 5: SET: not found\n".as_bytes(),
        logs.as_slice(),
        "log content different than expected"
    );
}
