//! Pure domain types for sovereign compute workloads and execution substrates.

use crate::agent_isolation::SandboxStrategy;
use serde::{Deserialize, Serialize};

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
            memory_bytes: 1024 * 1024 * 1024,    // 1 GiB
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
    /// Remote hybrid mesh node (joined via WireGuard/Headscale).
    RemoteHybridMesh,
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

/// Declarative specification for enrolling an external/hybrid compute runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRunnerSpec {
    pub node_name: String,
    pub vcpus: u32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub has_gpu: bool,
    pub gpu_model: Option<String>,
    pub wireguard_pubkey: String,
}

/// Pure plan to admit and configure a remote compute runner on the mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRunnerEnrollmentPlan {
    pub node_name: String,
    pub user: String,
    pub acl_tags: Vec<String>,
    pub expiration_seconds: u64,
    pub reusable: bool,
}
