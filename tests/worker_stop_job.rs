mod utils;

use claim::{assert_err, assert_ok};
use iron_exec::job::{Command, Status, StopType};
use utils::app::TestApp;
use uuid::Uuid;
#[test]
pub fn test_stop_success() {
    let mut app = TestApp::new();

    let test_cases = [
        (
            Command::new("sh", vec!["./tests/scripts/infinite_loop.sh".into()]),
            "kill an infinite loop",
            false,
        ),
        (
            Command::new("sh", vec!["./tests/scripts/long_runtime.sh".into()]),
            "kill a long running process",
            false,
        ),
        (
            Command::new("sh", vec!["./tests/scripts/infinite_loop.sh".into()]),
            "terminate an infinite loop",
            true,
        ),
        (
            Command::new("sh", vec!["./tests/scripts/long_runtime.sh".into()]),
            "terminate a long running process",
            true,
        ),
    ];

    for (i, (command, error_message, gracefully)) in test_cases.iter().enumerate() {
        let owner_id = Uuid::new_v4();
        let (job_id, wait_thread) = app.worker.start(command.clone(), owner_id).unwrap();

        assert_ok!(app.worker.stop(job_id, owner_id, *gracefully));

        assert_eq!(
            Status::Stopped(if *gracefully {
                StopType::Term
            } else {
                StopType::Kill
            }),
            *app.worker.jobs[i].status().lock().unwrap(),
            "failed to {}",
            error_message,
        );

        assert_ok!(wait_thread.join()).unwrap();
        app.log_handler
            .consume(format!("{}_{}.log", command.name(), job_id));
    }
}

#[test]
pub fn test_stop_error() {
    let mut app = TestApp::new();

    let job_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let (job, job_handle) = app
        .worker
        .start(
            Command::new("echo", vec!["hello".into(), "world".into()]),
            owner_id,
        )
        .unwrap();
    job_handle.join().unwrap().unwrap();

    let test_cases = [
        (
            job_id,
            Uuid::new_v4(),
            false,
            "kill a non-existent job",
            format!("no job with id {} found for user", job_id),
        ),
        (
            job,
            owner_id,
            false,
            "kill an exited process",
            "failed to send SIGKILL to job: ESRCH".into(),
        ),
        (
            job,
            owner_id,
            true,
            "terminate an exited process",
            "failed to send SIGTERM to job: ESRCH".into(),
        ),
    ];

    for (job_id, owner_id, gracefully, error_case, error_message) in test_cases {
        let stop_res = assert_err!(
            app.worker.stop(job_id, owner_id, gracefully),
            "stop did not error when trying to {}",
            error_case,
        );
        assert_eq!(
            error_message,
            stop_res.as_str(),
            "error message did not match expected message when trying to {}",
            error_case,
        );
    }

    app.log_handler.consume(format!("echo_{}.log", job));
}
