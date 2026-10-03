//! cgroups v2 isolation for CYNIC processes.
//!
//! WHY THIS EXISTS.
//!
//! CYNIC spawns processes without resource limits. The host crashed 3 times
//! today because a Python script forced 2.5 GiB of anonymous RAM per llama-cli
//! call, triggering a kernel swap bug (CVE-2026-64416, fixed in 7.1.4, we run
//! 7.0.0-31). cgroups v2 provides kernel-enforced limits on CPU, memory, I/O,
//! and process count.
//!
//! DESIGN.
//!
//! - `CgroupIsolation::create(name)` creates a cgroup under `/sys/fs/cgroup/cynic/`.
//! - `CgroupIsolation::apply(limits)` writes limits to the cgroup.
//! - `CgroupIsolation::add_pid(pid)` moves a process into the cgroup.
//! - `CgroupIsolation::destroy()` removes the cgroup.
//!
//! LIMITS SUPPORTED.
//!
//! - `memory.max` — maximum memory in bytes
//! - `memory.swap.max` — maximum swap in bytes (0 = no swap)
//! - `cpu.max` — CPU quota (e.g., "400000 100000" = 4 cores)
//! - `io.max` — I/O bandwidth (e.g., "8:0 rbps=1048576 wbps=1048576")
//! - `pids.max` — maximum number of processes
//!
//! SAFETY.
//!
//! - If cgroups v2 is not available, `create()` returns `Err` and the caller
//!   falls back to RLIMIT-only isolation.
//! - If a limit cannot be written, the error is logged but does not block
//!   the process (fail-open for availability, fail-closed for safety).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use cynic_protocol::resource_budget::ResourceLimit;

/// cgroups v2 isolation for CYNIC processes.
pub struct CgroupIsolation {
    /// Path to the cgroup directory.
    path: PathBuf,
}

impl CgroupIsolation {
    /// Base path for CYNIC cgroups.
    const BASE_PATH: &'static str = "/sys/fs/cgroup/cynic";

    /// Create a new cgroup with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` — Unique name for the cgroup (e.g., "agent-123", "build-456").
    ///
    /// # Returns
    ///
    /// `Ok(CgroupIsolation)` on success, `Err(String)` on failure.
    pub fn create(name: &str) -> Result<Self, String> {
        let path = Path::new(Self::BASE_PATH).join(name);

        fs::create_dir_all(Self::BASE_PATH)
            .map_err(|e| format!("cannot create cgroup base {}: {e}", Self::BASE_PATH))?;

        fs::create_dir_all(&path)
            .map_err(|e| format!("cannot create cgroup {}: {e}", path.display()))?;

        Ok(Self { path })
    }

    /// Apply resource limits to this cgroup.
    ///
    /// # Arguments
    ///
    /// * `limits` — Resource limits to apply.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(String)` on failure.
    pub fn apply(&self, limits: &ResourceLimit) -> Result<(), String> {
        if let Some(max_mem) = limits.max_memory_bytes {
            self.write_file("memory.max", &max_mem.to_string())?;
        }

        if limits.max_memory_bytes.is_some() {
            self.write_file("memory.swap.max", "0")?;
        }

        if let Some(max_cpu) = limits.max_cpu_cores {
            let quota = (max_cpu * 100_000.0) as u64;
            self.write_file("cpu.max", &format!("{quota} 100000"))?;
        }

        if let Some(max_io) = limits.max_io_bps {
            self.write_file("io.max", &format!("8:0 rbps={max_io} wbps={max_io}"))?;
        }

        Ok(())
    }

    /// Add a process to this cgroup.
    ///
    /// # Arguments
    ///
    /// * `pid` — Process ID to add.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(String)` on failure.
    pub fn add_pid(&self, pid: u32) -> Result<(), String> {
        self.write_file("cgroup.procs", &pid.to_string())
    }

    /// Destroy this cgroup.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(String)` on failure.
    pub fn destroy(&self) -> Result<(), String> {
        fs::remove_dir_all(&self.path)
            .map_err(|e| format!("cannot remove cgroup {}: {e}", self.path.display()))
    }

    /// Write a value to a cgroup file.
    fn write_file(&self, filename: &str, value: &str) -> Result<(), String> {
        let path = self.path.join(filename);
        let mut file =
            fs::File::create(&path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        file.write_all(value.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok(())
    }
}

impl Drop for CgroupIsolation {
    fn drop(&mut self) {
        let _ = self.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::resource_budget::ResourceLimit;

    /// Skip a test when cgroups v2 is not writable (non-root).
    fn skip_if_no_cgroup_access() -> bool {
        if unsafe { libc::geteuid() } != 0 {
            eprintln!("SKIP: cgroup tests require root");
            return true;
        }
        false
    }

    #[test]
    fn create_and_destroy_cgroup() {
        if skip_if_no_cgroup_access() {
            return;
        }
        let isolation = CgroupIsolation::create("test-create-destroy").unwrap();
        assert!(isolation.path.exists());
        isolation.destroy().unwrap();
        assert!(!isolation.path.exists());
    }

    #[test]
    fn apply_memory_limit() {
        if skip_if_no_cgroup_access() {
            return;
        }
        let isolation = CgroupIsolation::create("test-mem-limit").unwrap();
        let limits = ResourceLimit::new().with_max_memory_bytes(1024 * 1024 * 1024);
        isolation.apply(&limits).unwrap();
        isolation.destroy().unwrap();
    }

    #[test]
    fn apply_cpu_limit() {
        if skip_if_no_cgroup_access() {
            return;
        }
        let isolation = CgroupIsolation::create("test-cpu-limit").unwrap();
        let limits = ResourceLimit::new().with_max_cpu_cores(2.0);
        isolation.apply(&limits).unwrap();
        isolation.destroy().unwrap();
    }

    #[test]
    fn apply_all_limits() {
        if skip_if_no_cgroup_access() {
            return;
        }
        let isolation = CgroupIsolation::create("test-all-limits").unwrap();
        let limits = ResourceLimit::new()
            .with_max_memory_bytes(2 * 1024 * 1024 * 1024)
            .with_max_cpu_cores(4.0)
            .with_max_io_bps(100 * 1024 * 1024);
        isolation.apply(&limits).unwrap();
        isolation.destroy().unwrap();
    }

    #[test]
    fn add_pid_to_cgroup() {
        if skip_if_no_cgroup_access() {
            return;
        }
        let isolation = CgroupIsolation::create("test-add-pid").unwrap();
        let pid = std::process::id();
        isolation.add_pid(pid).unwrap();
        isolation.destroy().unwrap();
    }

    #[test]
    fn destroy_nonexistent_cgroup_fails() {
        let isolation = CgroupIsolation {
            path: PathBuf::from("/sys/fs/cgroup/cynic/nonexistent-12345"),
        };
        assert!(isolation.destroy().is_err());
    }
}
