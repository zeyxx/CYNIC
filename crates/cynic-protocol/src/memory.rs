//! Universal contract types for CYNIC memory observation.
//!
//! No system/process execution here — only the shared vocabulary for
//! observe/plan/apply/proof. Lives in `cynic-protocol` (not `cynic-host`) so the
//! kernel, node, host, and CLI surfaces all reference one canonical definition
//! with never a circular dependency.
//!
//! Memory vertical (4 levels, observed bottom-up):
//!   1. Physical  — `/proc/meminfo` MemTotal + DIMM/hugepage reservation hints
//!   2. Volume    — page size, swappiness, hugepage/THP config
//!   3. Logical   — `free`/meminfo split: used/buffers/cached/anon/reclaimable
//!   4. Pressure  — `/proc/pressure/memory` some/full stalls

use serde::{Deserialize, Serialize};

/// Memory pressure as reported by `/proc/pressure/memory`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryPressure {
    /// "some": at least one process is waiting for memory.
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub some_avg300: f64,
    pub some_total_us: u64,
    /// "full": all non-idle tasks stalled (reclaim/compaction exhausted).
    pub full_avg10: f64,
    pub full_avg60: f64,
    pub full_avg300: f64,
    pub full_total_us: u64,
}

/// Page/volume layer: huge-page and swap configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryVolume {
    /// Page size in bytes (always 4096 on x86_64; observed, not assumed).
    pub page_size_bytes: u64,
    /// Swappiness 0..100 (kernel reclaim bias toward swap).
    pub swappiness: Option<u64>,
    /// Number of hugepages reserved.
    pub hugepages_reserved: Option<u64>,
    /// Hugepage size in bytes (e.g. 2097152 = 2MiB).
    pub hugepage_size_bytes: Option<u64>,
}

/// A single memory zone/categoria as emitted by `free` or meminfo decomposition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Bytes — total addressable.
    pub total_bytes: u64,
    /// Bytes — currently unused, reclaimable without page reclaim stalls.
    pub free_bytes: u64,
    /// Bytes — available (free + reclaimable cache/buffers, minus shmem ceiling).
    pub available_bytes: u64,
    /// Bytes — used (total - free - buffers - cached).
    pub used_bytes: u64,
    /// Bytes — page cache + buffers reclaimable.
    pub reclaimable_bytes: u64,
}

/// Canonical memory observation report (schema `cynic.host_memory_report.v1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryReport {
    pub schema: String,
    pub timestamp: String,
    /// Level 1: physical memory (MemTotal / DIMM reservation).
    pub physical: MemoryUsage,
    /// Level 2: volume/page config (swappiness, hugepages).
    pub volume: MemoryVolume,
    /// Level 3: swap usage as a MemoryUsage (total/free/used).
    pub swap: MemoryUsage,
    /// Level 4: pressure stalls (Some/None) — unknown when /proc/pressure absent.
    #[serde(default)]
    pub pressure: Option<MemoryPressure>,
    /// Human-readable snapshot of the raw `/proc/meminfo` headline line used
    /// for forensic read-back (not a parsing authority, plain evidence).
    #[serde(default)]
    pub meminfo_headline: Option<String>,
    /// Explicit unknowns / skipped probes (e.g. pressure file missing).
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}

/// One node's memory report + status. `report: None` means unobserved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetMemoryNode {
    pub node_id: String,
    pub role: String,
    pub ip: String,
    pub outcome: String,
    #[serde(default)]
    pub report: Option<MemoryReport>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Fleet-wide rollup of memory observation (schema `cynic.fleet_memory.v1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetMemoryReport {
    pub schema: String,
    pub corpus_unix_s: i64,
    pub total_nodes: usize,
    pub observed_nodes: usize,
    pub failed_nodes: usize,
    pub nodes: Vec<FleetMemoryNode>,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}
