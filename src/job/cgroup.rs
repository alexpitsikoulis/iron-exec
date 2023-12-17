use std::{ffi::CString, fs::create_dir_all, io::Write, path::PathBuf, process::ExitStatus};

use nix::unistd::Pid;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CpuConfig {
    pub max: Option<String>,
    pub weight: Option<u16>,
}

impl CpuConfig {
    pub fn new(max: Option<String>, weight: Option<u16>) -> Option<Self> {
        if max.is_some() || weight.is_some() {
            Some(Self { max, weight })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct MemConfig {
    pub max: Option<String>,
    pub weight: Option<u16>,
}

impl MemConfig {
    pub fn new(max: Option<String>, weight: Option<u16>) -> Option<Self> {
        if max.is_some() || weight.is_some() {
            Some(Self { max, weight })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct IoConfig {
    pub max: Option<String>,
    pub weight: Option<u16>,
}

impl IoConfig {
    pub fn new(max: Option<String>, weight: Option<u16>) -> Option<Self> {
        if max.is_some() || weight.is_some() {
            Some(Self { max, weight })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct CgroupConfig {
    cpu: Option<CpuConfig>,
    memory: Option<MemConfig>,
    io: Option<IoConfig>,
    root_path: PathBuf,
}

impl CgroupConfig {
    pub fn new(
        cpu_max: Option<String>,
        cpu_weight: Option<u16>,
        memory_max: Option<String>,
        memory_weight: Option<u16>,
        io_max: Option<String>,
        io_weight: Option<u16>,
    ) -> Self {
        CgroupConfig {
            cpu: CpuConfig::new(cpu_max, cpu_weight),
            memory: MemConfig::new(memory_max, memory_weight),
            io: IoConfig::new(io_max, io_weight),
            root_path: PathBuf::new(),
        }
    }

    pub fn init(&mut self, command_name: &str, job_id: Uuid) -> Result<(), std::io::Error> {
        self.root_path = self
            .root_path
            .join(&format!("/sys/fs/cgroup/{}_{}", command_name, job_id));
        create_dir_all(self.root_path.clone())?;

        let _ = std::process::Command::new("mount")
            .arg("-t")
            .arg("cgroup2")
            .arg("none")
            .arg("/sys/fs/cgroup")
            .spawn()
            .unwrap()
            .wait();

        if let Some(cpu) = self.cpu.clone() {
            if let Some(cpu_max) = cpu.max {
                std::fs::write(self.root_path.join("cpu.max"), cpu_max.to_string())?;
            }
            if let Some(cpu_weight) = cpu.weight {
                std::fs::write(self.root_path.join("cpu.weight"), cpu_weight.to_string())?;
            }
        }

        if let Some(memory) = self.memory.clone() {
            if let Some(memory_max) = memory.max {
                std::fs::write(self.root_path.join("memory.max"), memory_max.to_string())?;
            }
            if let Some(memory_weight) = memory.weight {
                std::fs::write(
                    self.root_path.join("memory.weight"),
                    memory_weight.to_string(),
                )?;
            }
        }

        if let Some(io) = self.io.clone() {
            if let Some(io_max) = io.max {
                std::fs::write(self.root_path.join("io.max"), io_max.to_string())?;
            }

            if let Some(io_weight) = io.weight {
                std::fs::write(self.root_path.join("io.weight"), io_weight.to_string())?;
            }
        }

        std::fs::File::create(self.root_path.join("cgroup.procs"))?;

        Ok(())
    }

    pub fn add_process(&self, pid: Pid) -> Result<(), std::io::Error> {
        let mut procs_file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(self.root_path.join("cgroup.procs"))?;
        writeln!(procs_file, "{}", pid)?;
        Ok(())
    }

    pub fn delete(&self) -> Result<(), std::io::Error> {
        std::process::Command::new("umount").arg("cgroup2").spawn().unwrap().wait()?;
        std::fs::remove_dir_all(self.root_path.clone())
    }
}
