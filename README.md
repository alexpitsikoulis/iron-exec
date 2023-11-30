# iron-exec

## Summary
A Rust library for the execution of arbitrary Linux processes.

## Overview
A generic, reusable Rust library that acts as a worker to exexcute, monitor, and stop arbitrary Linux processes. This library is responsible for starting and stopping processes as well as streaming process output and handling process errors.
This library will use a map in local memory to store the statuses of running, completed, and failed jobs.
Processes are started by initializing a new Linux process using the ```std::process::Command``` struct. Processes are handled in parallel by dedicated ```Worker``` nodes per process.
The library will stream the output of the job to a log file. The log files' default location will be ```/tmp/{command_name}_{process_id}.log```. It will do so by reading from the process' stdout and stderr and pushing the output to buffered channels. Each channel will be fed lines of output from their respective source and a separate goroutine will listen on those channels and pipe them in chronological order into the log file. As this will cause a buildup of old log files, eventually a new stragey will be implemented to combat that. Whether that will be purging based on expiry by age or by disk space usage is yet to be determined. Another approach to be considered is the use of S3 buckets.
Upon requests to the API to stream output from a process, the content from the log file will be read to the terminal, tailing the logs as a process is running, seeing the logs in real time. This is achieved by reading the bytes of the file to stdout, when the end of the file is reached it will then wait for filesystem notifications (listened for by the ```notify``` crate) regarding changes to that log file. Streaming then resumes to output the new bytes and waits again. This continues until the process exits, at which point the stream loop is closed and the reader destroyed.
Upon requests to the API to stop a process, the library will locate the process associated with the provided ID and send a ```SIGKILL``` signal to stop the process forcefully.
The library uses cgroups to limit CPU, memory, and disk IO resources. The cgroup limitations will be applied by creating cgroups in the Linux filesystem. For example starting a process will create a cgroup at ```/sys/fs/cgroup/{command_name}_{process_id}```. A ```cgroup.controllers``` file is added to the cgroup with the content ```cpu io memory cpuset```. The group values provided are echoed into the respective files within the new cgroup. In order to start the process in the new cgroup, the initial ```std::process::Command``` struct will be forked using ```nix::unistd::fork``` and the resulting child process will be added to the cgroup. The command of this child process will then be set to the original requested command and it will be executed in the designated cgroup.

Available cgroups:
* CPU
    * cpu.max
    * cpu.weight
* Memory
    * memory.max
* Disk IO (applied to all devices by default)
    * io.weight
    * io.max

```rust
// Command is the body of a request to start a process as received from the API or CLI
pub struct Command {
    // Base command name
    Name: &'static str,
    // List of arguments to the command
    Args: Vec<&'static str>,
}

// Process represents the process and its associated status data
pub struct Process {
    // Unique ID
    ID: uuid::Uuid,
    // Command to be executed
    Cmd: std::process::Command,
    // Status of the process.
    Status: Status,
    // ID of the client which owns this job
    OwnerID: uuid::Uuid,
}

pub enum ProcessState {
    UnknownState,
    Running,
    Exited,
}

// Status of the process.
pub struct Status {
    // System process ID, different from Process.ID
    PID: i16,
    // ExitCode of the exited process. In the case of a process which has not exited or was terminated by a signal this value will be -1
    ExitCode: i8,
    // State of the process, enum value to represent whether the process is running, stopped, exited, killed, or in an unknown state due to some error
    State: State,
}

// Worker defines the basic execution behavior of the job dispatcher
pub trait Worker {
    // Start creates a Linux process
    //    - command: execution command and its arguments
    // Returns process on successful start and error on unsuccessful start 
    fn Start(command: Command) -> (Process, Error);
    // Stop kills execution of the specified process
    //    - processID: Process identifier (Process.ID)
    // Returns error when process stop is unsuccessful
    fn Stop(process_id: &'static str) -> (Error)
    // Query a process to check its status
    //    - processID: Process identifier (Process.ID)
    // Returns status of specified process and error if an error was encountered
    fn Query(process_id: &'static str) -> (Status, Error)
    // Streams the process output
    //    - ctx: context to cancel the log stream
    //    - processID: Process identifier (Process.ID)
    // Returns a ReadCloser to stream output to stdout/stderr and an error if an error was encountered
    fn Stream(ctx: std::task::Context, process_id: &'static str) -> (std::io::BufReader, Error)
}
```