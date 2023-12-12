use std::path::Path;

use iron_exec::job::{CgroupConfig, Job};

pub fn check_cgroup_files(job: Box<Job>, config: CgroupConfig) -> Result<(), &'static str> {
    let root_cgroup_path = format!("/sys/fs/cgroup/{}_{}", job.cmd().name(), job.id());
    let root_cgroup_path = Path::new(&root_cgroup_path);
    if std::fs::read_dir(root_cgroup_path).is_err() {
        return Err("root cgroup directory");
    };

    if let Some(cpu_max) = config.cpu_max() {
        if !std::fs::read_to_string(root_cgroup_path.join("cpu.max")).is_ok_and(|v| v == cpu_max.to_string()) {
            return Err("cpu max");
        }
    };

    if let Some(cpu_weight) = config.cpu_weight() {
        if !std::fs::read_to_string(root_cgroup_path.join("cpu.weight")).is_ok_and(|v| v == cpu_weight.to_string()) {
            return Err("cpu weight");
        }
    };

    if let Some(memory_max) = config.memory_max() {
        if !std::fs::read_to_string(root_cgroup_path.join("memory.max")).is_ok_and(|v| v == memory_max.to_string()) {
            return Err("memory max");
        }
    };

    if let Some(memory_weight) = config.memory_weight() {
        if !std::fs::read_to_string(root_cgroup_path.join("memory.weight")).is_ok_and(|v| v == memory_weight.to_string()) {
            return Err("memory weight");
        }
    };

    if let Some(io_max) = config.io_max() {
        if !std::fs::read_to_string(root_cgroup_path.join("io.max")).is_ok_and(|v| v == io_max.to_string()) {
            return Err("io max");
        }
    };

    if let Some(io_weight) = config.io_weight() {
        if !std::fs::read_to_string(root_cgroup_path.join("io.weight")).is_ok_and(|v| v == io_weight.to_string()) {
            return Err("io weight");
        }
    };

    if !std::fs::read_to_string(root_cgroup_path.join("tasks")).is_ok_and(|v| v == format!("{}", job.pid())) {
        return Err("tasks");
    };

    Ok(())
}