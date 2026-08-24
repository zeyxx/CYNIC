//! Universal contract types for CYNIC CPU observation.
//!
//! No system/process execution here — only the shared vocabulary for
//! observe/plan/apply/proof. Lives in `cynic_protocol` (not `cynic_host`) so the
//! kernel, node, host, and CLI surfaces all reference one canonical definition.
//!
//! CPU vertical (4 levels, observed bottom-up):
//!   1. Physical  — model name, socket/core/thread topology, max frequency
//!   2. Volume    — governor policy, cpu isolation mask, IRQ affinity
//!   3. Usage     — jiffies user/sys/idle/iowait + instantaneous util%
//!   4. Pressure  — `/proc/pressure/cpu` (some) + `/proc/softirqs` steal

use serde::{Deserialize, Serialize};

/// Physical CPU model + topology (level 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuPhysical {
    /// Model name from `/proc/cpuinfo` (e.g. "AMD EPYC 7R13 ...").
    pub model_name: String,
    /// Number of logical threads visible to the kernel.
    pub logical_cpus: usize,
    /// Number of physical cores (hardware threads / 2 on HT when available).
    pub cores: Option<usize>,
    /// Number of sockets (physical packages).
    pub sockets: Option<usize>,
    /// Max turbo/core frequency in Hz; `None` when unobservable.
    #[serde(default)]
    pub max_bogomips_or_hz: Option<f64>,
}

/// CPU policy / volume layer (level 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuVolume {
    /// Current scaling governor (e.g. "performance", "ondemand").
    #[serde(default)]
    pub governor: Option<String>,
    /// Isolation mask (`isolcpus=` kernel arg); empty when unset.
    #[serde(default)]
    pub isolated_mask: Option<String>,
    /// Number of online CPUs.
    pub online_cpus: usize,
    /// Number of possible CPUs (may exceed online).
    pub possible_cpus: usize,
}

/// CPU usage sample (level 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuUsage {
    /// User jiffies since boot.
    pub user: u64,
    /// System (kernel) jiffies since boot.
    pub system: u64,
    /// Idle jiffies since boot.
    pub idle: u64,
    /// I/O wait jiffies.
    pub iowait: u64,
    /// Softirq jiffies.
    pub softirq: u64,
    /// Steal (hypervisor) jiffies.
    pub steal: u64,
    /// Instantaneous utilisation 0..100 computed from the snapshot delta.
    #[serde(default)]
    pub util_pct: Option<f64>,
}

/// CPU pressure + contention signals (level 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuPressure {
    /// PSI "some": at least one task is stalled (10s avg fraction 0..1).
    #[serde(default)]
    pub some_avg10: Option<f64>,
    /// PSI "full" contention: all non-idle tasks stalled (10s avg fraction 0..1).
    #[serde(default)]
    pub full_avg10: Option<f64>,
    /// Softirq steal / dsiab workload indicator count.
    #[serde(default)]
    pub softirq_steal: Option<u64>,
}

/// Canonical CPU observation report (schema `cynic.host_cpu_report.v1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuReport {
    pub schema: String,
    pub timestamp: String,
    /// Level 1: physical model + topology.
    pub physical: CpuPhysical,
    /// Level 2: volume / policy layer.
    pub volume: CpuVolume,
    /// Level 3: usage snapshot (jiffies + util%).
    pub usage: CpuUsage,
    /// Level 4: pressure/fidelity signals; unknown when /proc/pressure absent.
    #[serde(default)]
    pub pressure: Option<CpuPressure>,
    /// Explicit unknowns / skipped probes.
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}

/// One node's CPU report + status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetCpuNode {
    pub node_id: String,
    pub role: String,
    pub ip: String,
    pub outcome: String,
    #[serde(default)]
    pub report: Option<CpuReport>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Fleet-wide rollup of CPU observation (schema `cynic.fleet_cpu.v1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetCpuReport {
    pub schema: String,
    pub corpus_unix_s: i64,
    pub total_nodes: usize,
    pub observed_nodes: usize,
    pub failed_nodes: usize,
    pub nodes: Vec<FleetCpuNode>,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}

pub const FLEET_CPU_SCHEMA: &str = "cynic.fleet_cpu.v1";
