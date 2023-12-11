mod utils;

use claim::{assert_err, assert_ok};
use iron_exec::job::{Command, Status, StopType};
use utils::app::TestApp;
use uuid::Uuid;

#[test]
pub fn test_query_success() {
    let mut app = TestApp::new();

    let test_cases = [
        (
            Command::new("sh", &["./tests/scripts/long_runtime.sh"]),
            Status::Running,
            None,
            "job is running",
            true,
        ),
        (
            Command::new("echo", &["hello", "world"]),
            Status::Exited(Some(0)),
            None,
            "job exited without error",
            false,
        ),
        (
            Command::new("sh", &["./tests/scripts/echo_and_error.sh"]),
            Status::Exited(Some(127)),
            None,
            "job exited with status 127",
            false,
        ),
        (
            Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
            Status::Stopped(StopType::Kill),
            Some(false),
            "job was killed",
            false,
        ),
        (
            Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
            Status::Stopped(StopType::Term),
            Some(true),
            "job was terminated",
            false,
        ),
    ];

    for (command, expected_status, gracefully, error_message, close_after) in test_cases {
        let (job, wait_handle) = app.queue_job(command, None);

        if let Some(gracefully) = gracefully {
            app.worker
                .stop(job.id(), job.owner_id(), gracefully)
                .unwrap();
            wait_handle.join().unwrap();
            let status = assert_ok!(
                app.worker.query(job.id(), job.owner_id()),
                "query request failed"
            );
            assert_eq!(
                expected_status, status,
                "job was not in expected state when {}",
                error_message
            );
        } else {
            if close_after {
                let status = assert_ok!(app.worker.query(job.id(), job.owner_id()));
                assert_eq!(
                    expected_status, status,
                    "job was not in expected state when {}",
                    error_message
                );
                app.worker.stop(job.id(), job.owner_id(), false).unwrap();
                wait_handle.join().unwrap();
            } else {
                wait_handle.join().unwrap();
                let status = assert_ok!(app.worker.query(job.id(), job.owner_id()));
                assert_eq!(
                    expected_status, status,
                    "job was not in expected state when {}",
                    error_message
                );
            }
        }

        app.log_handler
            .consume(format!("{}_{}.log", job.cmd().name(), job.id()));
    }
}

#[test]
pub fn test_query_error() {
    let mut app = TestApp::new();

    let job_id = Uuid::new_v4();
    let (job, job_handle) = app.queue_job(Command::new("echo", &["hello", "world"]), None);
    job_handle.join().unwrap();

    let test_cases = [
        (job_id, Uuid::new_v4(), "query a non-existent job"),
        (
            job.id(),
            Uuid::new_v4(),
            "query existing job with wrong owner_id",
        ),
    ];

    for (job_id, owner_id, error_case) in test_cases {
        let error = assert_err!(
            app.worker.query(job_id, owner_id),
            "query did not error when trying to {}",
            error_case,
        );

        assert_eq!(
            format!("no job with id {} found for user", job_id),
            error.as_str(),
            "error message did not match expected message",
        );
    }

    app.log_handler.consume(format!("echo_{}.log", job.id()));
}