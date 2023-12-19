mod utils;

use std::{io::Read, thread, time::Duration};

use claim::{assert_err, assert_ok};
use iron_exec::job::Command;
use utils::{app::TestApp, logs::LOG_DIR};
use uuid::Uuid;

#[test]
pub fn test_stream_job_succes() {
    let mut app = TestApp::new();

    let test_cases = [
        (
            Command::new("echo".into(), vec!["hello".into(), "world".into()]),
            false,
            "job exited successfully",
        ),
        (
            Command::new("sh".into(), vec!["./tests/scripts/infinite_loop.sh".into()]),
            true,
            "job loops infinitely",
        ),
    ];

    for (command, ongoing, error_case) in test_cases {
        let owner_id = Uuid::new_v4();
        let (job_id, job_handle) = app.worker.start(command.clone(), owner_id).unwrap();
        let log_filename = format!("{}_{}.log", command.name(), job_id);
        let log_filepath = format!("{}/{}", LOG_DIR, log_filename);
        let mut reader = assert_ok!(
            app.worker.stream(job_id, owner_id),
            "job stream failed to return log file reader when {}",
            error_case,
        );

        if !ongoing {
            job_handle.join().unwrap().unwrap();

            let file_content = std::fs::read(log_filepath).unwrap();

            let mut buf = Vec::new();
            let _ = assert_ok!(
                reader.read_to_end(&mut buf),
                "failed to read from BufReader",
            );

            assert_eq!(
                file_content.as_slice(),
                buf,
                "buffer contents did not match log file content",
            );
        } else {
            let mut buf = Vec::new();

            for _ in 0..5 {
                thread::sleep(Duration::from_secs(1));
                let _ = assert_ok!(
                    reader.read_to_end(&mut buf),
                    "failed to read from BufReader",
                );

                let file_content = std::fs::read(&log_filepath).unwrap();
                assert_eq!(file_content, buf, "BufReader did not update with log file",);
            }

            app.worker.stop(job_id, owner_id, false).unwrap();
            job_handle.join().unwrap().unwrap();
        }
        app.log_handler.consume(log_filename);
    }
}

#[test]
pub fn test_stream_job_error() {
    let mut app = TestApp::new();

    let job_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let (job, job_handle) = app
        .worker
        .start(
            Command::new("echo".into(), vec!["hello".into(), "world".into()]),
            owner_id,
        )
        .unwrap();
    job_handle.join().unwrap().unwrap();

    let test_cases = [
        (job_id, Uuid::new_v4(), "stream a non-existent job", format!("no job with id {} found for user", job_id), false),
        (job, Uuid::new_v4(), "stream a job the current user does not own", format!("no job with id {} found for user", job), false),
        (job, owner_id, "stream a job where the log file has been deleted", "failed to open log file: Os { code: 2, kind: NotFound, message: \"No such file or directory\" }".into(), true),
    ];

    for (job_id, owner_id, error_case, error_message, logs_deleted) in test_cases {
        if logs_deleted {
            app.log_handler.consume(format!("echo_{}.log", job_id));
        }

        let e = assert_err!(
            app.worker.stream(job_id, owner_id),
            "stream job did not error when trying to {}",
            error_case,
        );

        assert_eq!(
            error_message,
            e.as_str(),
            "error message did not match expected message when {}",
            error_case,
        );
    }
}
