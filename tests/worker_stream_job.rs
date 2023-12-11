mod utils;

use std::{
    io::Read,
    thread,
    time::Duration,
};

use claim::assert_ok;
use iron_exec::job::Command;
use utils::{app::TestApp, logs::LOG_DIR};

#[test]
pub fn test_stream_job_succes() {
    let mut app = TestApp::new();

    let test_cases = [(
        Command::new("echo", &["hello", "world"]),
        false,
        "job exited successfully",
    ),
    (
        Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
        true,
        "job loops infinitely",
    )];

    for (command, ongoing, error_case) in test_cases {
        let (job, job_handle) = app.queue_job(command, None);
        let log_filename = format!("{}_{}.log", command.name(), job.id());
        let log_filepath = format!("{}/{}", LOG_DIR, log_filename);
        let mut reader = assert_ok!(
            app.worker.stream(job.id(), job.owner_id()),
            "job stream failed to return log file reader when {}",
            error_case,
        );

        if !ongoing {
            job_handle.join().unwrap();

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

            for x in 0..5 {
                thread::sleep(Duration::from_secs(1));
                let _ = assert_ok!(
                    reader.read_to_end(&mut buf),
                    "failed to read from BufReader",
                );

                let file_content = std::fs::read(&log_filepath).unwrap();
                assert_eq!(
                    file_content,
                    buf,
                    "BufReader did not update with log file",
                );
            }

            app.worker.stop(job.id(), job.owner_id(), false).unwrap();
            job_handle.join().unwrap();
        }
        app.log_handler.consume(log_filename);
    }
}
