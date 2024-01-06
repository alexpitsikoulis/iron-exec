# iron-exec

## Summary
A Rust library for the execution of arbitrary Linux processes.

## Overview
A generic, reusable Rust library that acts as a worker to exexcute, monitor, and stop arbitrary Linux processes. This library is responsible for starting and stopping processes as well as streaming process output and handling process errors.
This library will use a map in local memory to store the statuses of running, completed, and failed jobs.
Processes are started by initializing a new Linux process using the ```std::process::Command``` struct. Processes are handled in parallel by dedicated ```Worker``` nodes per process.
The library will stream the output of the job to a log file. The log files' default location will be ```/tmp/{command_name}_{process_id}.log```. It will do so by reading from the process' stdout and stderr and pushing the output to buffered channels. Each channel will be fed lines of output from their respective source and a separate routine will listen on those channels and pipe them in chronological order into the log file. As this will cause a buildup of old log files, eventually a new stragey will be implemented to combat that. Whether that will be purging based on expiry by age or by disk space usage is yet to be determined. Another approach to be considered is the use of S3 buckets.
Upon requests to the API to stream output from a process, the content from the log file will be read to the terminal, tailing the logs as a process is running, seeing the logs in real time. This is achieved by reading the bytes of the file to stdout, when the end of the file is reached it will then wait for filesystem notifications (listened for by the ```notify``` crate) regarding changes to that log file. Streaming then resumes to output the new bytes and waits again. This continues until the process exits, at which point the stream loop is closed and the reader destroyed.
Upon requests to the API to stop a process, the library will locate the process associated with the provided ID and send a ```SIGKILL``` signal to stop the process forcefully.

```rust
// Command is the body of a request to start a process as received from the API or CLI
pub struct Command {
    // Base command name
    name: String,
    // List of arguments to the command
    args: Vec<String>,
}

// Job represents the process and its associated status data
pub struct Job {
    // Unique ID
    id: uuid::Uuid,
    // Command to be executed
    cmd: Command,
    // System pid of the running process
    pid: u32,
    // Status of the job.
    status: Arc<Mutex<Status>>,
    // ID of the client which owns this job
    owner_id: uuid::Uuid,
}

// JobInfo is the struct returned from the Query method
pub struct JobInfo {
    // Unique ID
    id: uuid::Uuid,
    // Command to be executed
    cmd: Command,
    // System pid of the running process
    pid: u32,
    // Status of the job.
    owner_id: uuid::Uuid,
}

// Status of the process
pub enum Status {
    UnknownState,
    Running,
    Exited,
    // Exited with optional exit code
    Exited(Option<i32>),
    // Stopped with Terminated or Killed StopType
    Stopped(StopType),
}

// Enum to denote whether a process was killed (SIGKILL) or terminated (SIGTERM)
pub enum StopType {
    Term,
    Kill,
}


// Worker defines the basic execution behavior of the job dispatcher
pub struct Worker {
    // List of jobs to be executed by the worker
    jobs: Vec<Arc<Job>>,
}

impl Worker {
    // Start creates a Linux process
    //    - command: execution command and its arguments
    //    - owner_id: UUID of the user starting the job
    // Returns job ID on successful start and error on unsuccessful start 
    fn start(&mut self, command: Command, owner_id: uuid::Uuid) -> Result<uuid::Uuid, Error>;
    // Stop kills execution of the specified job
    //    - job_id: Job identifier (Job.ID)
    //    - owner_id: User identifier (Job.owner_id)
    // Returns error when job stop is unsuccessful
    fn stop(&mut self, job_id: uuid::Uuid, owner_id: uuid::Uuid) -> Result<(), Error>;
    // Query a job to check its status
    //    - job_id: Job identifier (Job.id)
    //    - owner_id: User identifier (Job.owner_id)
    // Returns status of specified job and error if an error was encountered
    fn query(&self, job_id: uuid::Uuid, owner_id: uuid::Uuid, gracefully: bool) -> Result<JobInfo, Error>;
    // Streams the job output
    //    - job_id: Job identifier (Job.id)
    //    - owner_id: User identifier (Job.owner_id)
    // Returns a std::io::BufReader to stream output to stdout/stderr and an error if an error was encountered
    fn stream(&self, job_id: uuid::Uuid, owner_id: uuid::Uuid) -> Result<std::io::BufReader<File>, Error>;
}
```