use std::{
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use super::Job;

pub struct CgroupConfig {
    cpu_max: Option<u32>,
    cpu_weight: Option<u16>,
    memory_max: Option<u32>,
    memory_weight: Option<u32>,
    io_max: Option<u32>,
    io_weight: Option<u16>,
}

impl CgroupConfig {
    pub fn new(
        job: Box<Job>,
        cpu_max: Option<u32>,
        cpu_weight: Option<u16>,
        memory_max: Option<u32>,
        memory_weight: Option<u32>,
        io_max: Option<u32>,
        io_weight: Option<u16>,
    ) -> Result<(), std::io::Error> {
        let path_str = &format!("/sys/fs/cgroup/{}_{}", job.cmd().name(), job.id());
        let root_cgroup_path = Path::new(path_str);
        create_dir_all(root_cgroup_path)?;

        if let Some(cpu_max) = cpu_max {
            std::fs::write(root_cgroup_path.join("cpu.max"), cpu_max.to_string())?;
        }

        if let Some(cpu_weight) = cpu_weight {
            std::fs::write(root_cgroup_path.join("cpu.weight"), cpu_weight.to_string())?;
        }

        if let Some(memory_max) = memory_max {
            std::fs::write(root_cgroup_path.join("memory.max"), memory_max.to_string())?;
        }

        if let Some(memory_weight) = memory_weight {
            std::fs::write(
                root_cgroup_path.join("memory.weight"),
                memory_weight.to_string(),
            )?;
        }

        if let Some(io_max) = io_max {
            std::fs::write(root_cgroup_path.join("io.max"), io_max.to_string())?;
        }

        if let Some(io_weight) = io_weight {
            std::fs::write(root_cgroup_path.join("io.weight"), io_weight.to_string())?;
        }
        Ok(())
    }
}

pub fn create_cgroup(
    cmd_name: &'static str,
    job_id: Uuid,
    config: CgroupConfig,
) -> Result<PathBuf, std::io::Error> {
    let path_str = &format!("/sys/fs/cgroup/{}_{}", cmd_name, job_id);
    let root_cgroup_path = Path::new(path_str);
    create_dir_all(root_cgroup_path)?;

    if let Some(cpu_max) = config.cpu_max {
        std::fs::write(root_cgroup_path.join("cpu.max"), cpu_max.to_string())?;
    }

    if let Some(cpu_weight) = config.cpu_weight {
        std::fs::write(root_cgroup_path.join("cpu.weight"), cpu_weight.to_string())?;
    }

    if let Some(memory_max) = config.memory_max {
        std::fs::write(root_cgroup_path.join("memory.max"), memory_max.to_string())?;
    }

    if let Some(memory_weight) = config.memory_weight {
        std::fs::write(
            root_cgroup_path.join("memory.weight"),
            memory_weight.to_string(),
        )?;
    }

    if let Some(io_max) = config.io_max {
        std::fs::write(root_cgroup_path.join("io.max"), io_max.to_string())?;
    }

    if let Some(io_weight) = config.io_weight {
        std::fs::write(root_cgroup_path.join("io.weight"), io_weight.to_string())?;
    }

    Ok(root_cgroup_path.to_owned())
}

pub fn add_to_cgroup(pid: u32, cgroup_path: PathBuf) -> Result<(), std::io::Error> {
    let mut tasks_file = OpenOptions::new()
        .write(true)
        .open(cgroup_path.join("tasks"))
        .unwrap_or_else(|_| {
            panic!(
                "failed to open cgroups file '{:?}'",
                cgroup_path.join("tasks")
            )
        });

    writeln!(tasks_file, "{}", pid)
}
