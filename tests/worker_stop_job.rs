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
            Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
            "kill an infinite loop",
            false,
        ),
        (
            Command::new("sh", &["./tests/scripts/long_runtime.sh"]),
            "kill a long running process",
            false,
        ),
        (
            Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
            "terminate an infinite loop",
            true,
        ),
        (
            Command::new("sh", &["./tests/scripts/long_runtime.sh"]),
            "terminate a long running process",
            true,
        ),
    ];

    for (i, (command, error_message, gracefully)) in test_cases.iter().enumerate() {
        let (job, wait_thread) = app.queue_job(*command, None);

        assert_ok!(app.worker.stop(job.id(), job.owner_id(), *gracefully));

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

        assert_ok!(wait_thread.join());
        app.log_handler
            .consume(format!("{}_{}.log", job.cmd().name(), job.id()));
    }
}

#[test]
pub fn test_stop_error() {
    let mut app = TestApp::new();

    let job_id = Uuid::new_v4();
    let (job, job_handle) = app.queue_job(Command::new("echo", &["hello", "world"]), None);
    job_handle.join().unwrap();

    let test_cases = [
        (
            job_id,
            Uuid::new_v4(),
            false,
            "kill a non-existent job",
            format!("no job with id {} found for user", job_id),
        ),
        (
            job.id(),
            job.owner_id(),
            false,
            "kill an exited process",
            "failed to send SIGKILL to job: ESRCH".into(),
        ),
        (
            job.id(),
            job.owner_id(),
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
            "error message did not match expected message",
        );
    }

    app.log_handler.consume(format!("echo_{}.log", job.id()));
}
