//! Pure domain types for sovereign compute workloads and execution substrates.

use serde::{Deserialize, Serialize};
use crate::agent_isolation::SandboxStrategy;

/// Resource and isolation requirements declared by a workload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeRequirements {
    pub vcpus: u32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub requires_gpu: bool,
    pub preferred_strategy: SandboxStrategy,
    pub allowed_strategies: Vec<SandboxStrategy>,
}

impl Default for ComputeRequirements {
    fn default() -> Self {
        Self {
            vcpus: 1,
            memory_bytes: 1024 * 1024 * 1024, // 1 GiB
            disk_bytes: 10 * 1024 * 1024 * 1024, // 10 GiB
            requires_gpu: false,
            preferred_strategy: SandboxStrategy::BwrapEphemeral,
            allowed_strategies: vec![
                SandboxStrategy::BwrapEphemeral,
                SandboxStrategy::LxcUnprivileged,
            ],
        }
    }
}

/// The underlying substrate realizing the compute environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubstrateKind {
    /// Remote/local Proxmox VE LXC container.
    ProxmoxLxc,
    /// Local bubblewrap process isolation (non-root).
    LocalBwrap,
    /// In-memory mock for testing.
    Mock,
}

/// Status of an allocated runner instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerStatus {
    Allocated,
    Running,
    Drained,
    Terminated,
}

/// Handle representing an allocated runner instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerAllocation {
    pub allocation_id: String,
    pub workload_id: String,
    pub substrate: SubstrateKind,
    pub strategy: SandboxStrategy,
    pub status: RunnerStatus,
    pub network_endpoint: Option<String>,
}

/// Output of a command executed inside an allocated runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}
