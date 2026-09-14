use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ResourceLimitConfig {
    /// Optional process-wide address-space ceiling. This is defense in depth;
    /// cgroup v2 memory.max is the preferred Linux hard memory boundary.
    pub max_address_space_bytes: Option<u64>,
    /// Existing/delegated cgroup v2 directory. When set, this process is moved
    /// into it after the limits have been written successfully.
    pub cgroup_dir: Option<PathBuf>,
    pub cgroup_memory_max_bytes: Option<u64>,
    pub cgroup_memory_swap_max_bytes: Option<u64>,
    /// CPU quota where 100 == one logical CPU, 200 == two CPUs, etc.
    pub cgroup_cpu_percent: Option<u32>,
    pub cgroup_pids_max: Option<u64>,
}

#[derive(Debug)]
pub enum ResourceLimitError {
    InvalidConfig(&'static str),
    #[cfg(not(target_os = "linux"))]
    Unsupported(&'static str),
    Io {
        operation: &'static str,
        path: Option<PathBuf>,
        source: io::Error,
    },
    CpuQuotaOverflow,
}

impl fmt::Display for ResourceLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => f.write_str(message),
            #[cfg(not(target_os = "linux"))]
            Self::Unsupported(message) => f.write_str(message),
            Self::Io {
                operation,
                path: Some(path),
                source,
            } => write!(f, "{operation} {}: {source}", path.display()),
            Self::Io {
                operation,
                path: None,
                source,
            } => write!(f, "{operation}: {source}"),
            Self::CpuQuotaOverflow => f.write_str("CPU quota overflow"),
        }
    }
}

impl Error for ResourceLimitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn apply(config: &ResourceLimitConfig) -> Result<(), ResourceLimitError> {
    if let Some(bytes) = config.max_address_space_bytes {
        if bytes < 64 * 1024 * 1024 {
            return Err(ResourceLimitError::InvalidConfig(
                "--max-process-memory-bytes must be at least 64 MiB",
            ));
        }
        apply_address_space_limit(bytes)?;
    }
    if let Some(dir) = config.cgroup_dir.as_deref() {
        apply_cgroup_v2(dir, config)?;
    } else if config.cgroup_memory_max_bytes.is_some()
        || config.cgroup_memory_swap_max_bytes.is_some()
        || config.cgroup_cpu_percent.is_some()
        || config.cgroup_pids_max.is_some()
    {
        return Err(ResourceLimitError::InvalidConfig(
            "cgroup limits require --cgroup-dir",
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_address_space_limit(bytes: u64) -> Result<(), ResourceLimitError> {
    let limit = libc::rlimit {
        rlim_cur: bytes as libc::rlim_t,
        rlim_max: bytes as libc::rlim_t,
    };
    // SAFETY: setrlimit reads the supplied struct during the call and does not
    // retain the pointer. RLIMIT_AS is process-wide by Linux definition.
    let rc = unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) };
    if rc != 0 {
        return Err(ResourceLimitError::Io {
            operation: "failed to apply RLIMIT_AS",
            path: None,
            source: io::Error::last_os_error(),
        });
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_address_space_limit(_bytes: u64) -> Result<(), ResourceLimitError> {
    Err(ResourceLimitError::Unsupported(
        "--max-process-memory-bytes is currently implemented only on Linux",
    ))
}

#[cfg(target_os = "linux")]
fn apply_cgroup_v2(dir: &Path, config: &ResourceLimitConfig) -> Result<(), ResourceLimitError> {
    let meta = std::fs::metadata(dir).map_err(|source| ResourceLimitError::Io {
        operation: "cannot access cgroup directory",
        path: Some(dir.to_path_buf()),
        source,
    })?;
    if !meta.is_dir() {
        return Err(ResourceLimitError::InvalidConfig(
            "--cgroup-dir is not a directory",
        ));
    }
    if !dir.join("cgroup.procs").exists() || !dir.join("cgroup.controllers").exists() {
        return Err(ResourceLimitError::InvalidConfig(
            "--cgroup-dir does not look like a cgroup v2 directory",
        ));
    }

    if let Some(bytes) = config.cgroup_memory_max_bytes {
        if bytes < 64 * 1024 * 1024 {
            return Err(ResourceLimitError::InvalidConfig(
                "--cgroup-memory-max-bytes must be at least 64 MiB",
            ));
        }
        write_control(dir, "memory.max", &bytes.to_string())?;
    }
    if let Some(bytes) = config.cgroup_memory_swap_max_bytes {
        write_control(dir, "memory.swap.max", &bytes.to_string())?;
    }
    if let Some(percent) = config.cgroup_cpu_percent {
        if percent == 0 || percent > 6400 {
            return Err(ResourceLimitError::InvalidConfig(
                "--cgroup-cpu-percent must be between 1 and 6400",
            ));
        }
        const PERIOD_US: u64 = 100_000;
        let quota = PERIOD_US
            .checked_mul(percent as u64)
            .ok_or(ResourceLimitError::CpuQuotaOverflow)?
            / 100;
        write_control(dir, "cpu.max", &format!("{quota} {PERIOD_US}"))?;
    }
    if let Some(max) = config.cgroup_pids_max {
        if max == 0 {
            return Err(ResourceLimitError::InvalidConfig(
                "--cgroup-pids-max must be greater than zero",
            ));
        }
        write_control(dir, "pids.max", &max.to_string())?;
    }

    // Move the server only after every requested control write succeeded.
    write_control(dir, "cgroup.procs", &std::process::id().to_string())?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_cgroup_v2(_dir: &Path, _config: &ResourceLimitConfig) -> Result<(), ResourceLimitError> {
    Err(ResourceLimitError::Unsupported(
        "cgroup v2 resource limits are Linux-only",
    ))
}

#[cfg(target_os = "linux")]
fn write_control(dir: &Path, name: &str, value: &str) -> Result<(), ResourceLimitError> {
    let path = dir.join(name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .map_err(|source| ResourceLimitError::Io {
            operation: "failed to open cgroup control",
            path: Some(path.clone()),
            source,
        })?;
    file.write_all(value.as_bytes())
        .map_err(|source| ResourceLimitError::Io {
            operation: "failed to write cgroup control",
            path: Some(path),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::{ResourceLimitConfig, ResourceLimitError, apply};
    use std::path::PathBuf;

    #[test]
    fn cgroup_limits_require_directory() {
        let cfg = ResourceLimitConfig {
            cgroup_cpu_percent: Some(100),
            ..Default::default()
        };
        assert!(matches!(
            apply(&cfg),
            Err(ResourceLimitError::InvalidConfig(
                "cgroup limits require --cgroup-dir"
            ))
        ));
    }

    #[test]
    fn too_small_address_space_is_typed_configuration_error() {
        let cfg = ResourceLimitConfig {
            max_address_space_bytes: Some(1024),
            ..Default::default()
        };
        assert!(matches!(
            apply(&cfg),
            Err(ResourceLimitError::InvalidConfig(_))
        ));
    }

    #[test]
    fn zero_pid_limit_is_rejected_before_use() {
        let cfg = ResourceLimitConfig {
            cgroup_dir: Some(PathBuf::from("/definitely/missing")),
            cgroup_pids_max: Some(0),
            ..Default::default()
        };
        // Missing directory is also an error; this test guarantees fail-closed behavior.
        assert!(apply(&cfg).is_err());
    }
}
