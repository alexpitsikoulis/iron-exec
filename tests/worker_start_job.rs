mod utils;

use claim::{assert_err, assert_ok};
use iron_exec::job::{Command, Status};
use utils::app::TestApp;
use uuid::Uuid;

#[test]
pub fn test_start_job_success() {
    let app = TestApp::new();

    let test_cases = [
        (Command::new("echo".into(), vec!["hello world".into()]), Status::Exited(Some(0)), "hello world\n", "job should exit successfully and write to stdout"),
        (Command::new("sh".into(), vec!["./tests/scripts/error.sh".into()]), Status::Exited(Some(127)), "./tests/scripts/error.sh: 1: SET: not found\n", "job should exit with 127 and write to stderr"),
        (Command::new("sh".into(), vec!["./tests/scripts/echo_and_error.sh".into()]), Status::Exited(Some(127)), "testing\none more\nstderr test\nback to stdout\n./tests/scripts/echo_and_error.sh: 5: SET: not found\n", "job should exit with 127 and write to stdout and stderr"),
    ];

    for (i, (command, expected_status, expected_log_content, error_case)) in
        test_cases.iter().enumerate()
    {
        let job_id = assert_ok!(app.worker.start(command.clone(), Uuid::new_v4(),));

        assert_eq!(
            i + 1,
            app.worker.jobs.lock().unwrap().len(),
            "job was not pushed to worker jobs vec when {}",
            error_case,
        );

        assert_ne!(
            0,
            app.worker.jobs.lock().unwrap().get(i).unwrap().pid(),
            "pid was not assigned to job when {}",
            error_case,
        );

        assert_ok!(app.wait());

        assert_eq!(
            *expected_status,
            *app.worker
                .jobs
                .lock()
                .unwrap()
                .get(i)
                .unwrap()
                .status()
                .lock()
                .unwrap(),
            "job did not exit with expected exit code when {}",
            error_case,
        );

        let logs = app
            .log_handler
            .consume(format!("{}_{}.log", command.name(), job_id));
        assert_eq!(
            expected_log_content.to_string(),
            String::from_utf8(logs).unwrap(),
            "job logs did not match expected content when {}",
            error_case,
        );
    }
}

#[test]
pub fn test_start_job_error() {
    let app = TestApp::new();

    let test_cases = [(
        Command::new("whatever-madeup-command".into(), vec![]),
        "failed to spawn child process: Os { code: 2, kind: NotFound, message: \"No such file or directory\" }",
        "job is started with invalid command",
    )];

    for (command, error_message, error_case) in test_cases {
        let e = assert_err!(
            app.worker.start(command, Uuid::new_v4()),
            "job did not error when {}",
            error_case
        );

        assert_eq!(error_message, e.as_str(),)
    }
}
