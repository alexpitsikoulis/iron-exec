use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use super::Job;

pub struct CgroupConfig {
    cpu_max: Option<u32>,
    cpu_weight: Option<u16>,
    memory_max: Option<u32>,
    memory_weight: Option<u16>,
    io_max: Option<u32>,
    io_weight: Option<u16>,
}

impl CgroupConfig {
    pub fn new(
        cpu_max: Option<u32>,
        cpu_weight: Option<u16>,
        memory_max: Option<u32>,
        memory_weight: Option<u16>,
        io_max: Option<u32>,
        io_weight: Option<u16>,
    ) -> Self {
        CgroupConfig {
            cpu_max,
            cpu_weight,
            memory_max,
            memory_weight,
            io_max,
            io_weight,
        }
    }

    pub fn init(&self, job: Box<Job>) -> Result<PathBuf, std::io::Error> {
        let path_str = &format!("/sys/fs/cgroup/{}_{}", job.cmd().name(), job.id());
        let root_cgroup_path = Path::new(path_str);
        create_dir_all(root_cgroup_path)?;

        if let Some(cpu_max) = self.cpu_max {
            std::fs::write(root_cgroup_path.join("cpu.max"), cpu_max.to_string())?;
        }

        if let Some(cpu_weight) = self.cpu_weight {
            std::fs::write(root_cgroup_path.join("cpu.weight"), cpu_weight.to_string())?;
        }

        if let Some(memory_max) = self.memory_max {
            std::fs::write(root_cgroup_path.join("memory.max"), memory_max.to_string())?;
        }

        if let Some(memory_weight) = self.memory_weight {
            std::fs::write(
                root_cgroup_path.join("memory.weight"),
                memory_weight.to_string(),
            )?;
        }

        if let Some(io_max) = self.io_max {
            std::fs::write(root_cgroup_path.join("io.max"), io_max.to_string())?;
        }

        if let Some(io_weight) = self.io_weight {
            std::fs::write(root_cgroup_path.join("io.weight"), io_weight.to_string())?;
        }
        Ok(root_cgroup_path.to_path_buf())
    }

    pub fn add_process(&self, path: PathBuf, pid: u32) -> Result<(), std::io::Error> {
        std::fs::write(path.join("tasks"), pid.to_string())
    }
}
