//! Pure shapes for OS substrate health observation.
//!
//! No I/O here. These structs are the authoritative representation of what was
//! observed (raw) and what was decided (findings). They are serializable so the
//! CLI can project them as machine-readable JSON for downstream read-back.

use serde::{Deserialize, Serialize};

/// Tunable authority for OS-health evaluation. Defaults come from
/// `super::default_os_health_config()`; operators may override thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsHealthConfig {
    /// If true, a kernel that will NOT panic on soft lockup is treated as a
    /// finding — the exact condition that let the 2026-08-18 lockup freeze the
    /// box until a hard reset (softlockup_panic=0).
    pub softlockup_panic_required: bool,
    /// THP `enabled` values considered risky (defragment-on-alloc pressure).
    pub thp_enabled_forbid: Vec<String>,
    /// THP `defrag` values considered risky (synchronous reclaim stalls).
    pub thp_defrag_forbid: Vec<String>,
    /// Below this much free memory (MiB) we flag memory pressure.
    pub available_mem_mib_floor: u64,
    /// Above this reclaim-pressure percentage we flag memory pressure.
    pub reclaim_pressure_pct_ceiling: u8,
}

/// Raw read-only observation of the OS substrate. Produced by `observe`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsObservation {
    pub softlockup_panic: bool,
    pub thp_enabled: String,
    pub thp_defrag: String,
    pub available_mem_mib: u64,
    pub reclaim_pressure_pct: u8,
    /// Count of soft-lockup lines seen in the accessible kernel log.
    pub dmesg_softlockup_count: u64,
    /// Count of hung-task lines seen in the accessible kernel log.
    pub dmesg_hung_task_count: u64,
    /// Count of OOM-killer lines seen in the accessible kernel log.
    pub dmesg_oom_count: u64,
}

/// One detected anomaly. Pure data — severity drives the assessment status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsHealthFinding {
    /// Stable kind, e.g. `softlockup_no_panic`, `thp_defrag_risky`.
    pub kind: String,
    pub severity: String, // "critical" | "warning" | "info"
    pub message: String,
    /// Machine-readable remediation intent (bounded, reviewable).
    pub remediation: String,
    /// Sovereign error code from `CYNIC-OSH-*` catalog (error_catalog),
    /// when the finding maps to a typed contract. None for pure informational
    /// findings that carry no canonical blocker.
    #[serde(default)]
    pub error_code: Option<String>,
}

/// Final assessment: the desired/typed observation plus the pure decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsHealthAssessment {
    pub schema: String,
    pub status: String, // "healthy" | "degraded" | "unhealthy"
    pub observed: OsObservation,
    pub findings: Vec<OsHealthFinding>,
    /// Explicit residual unknowns (e.g. kernel log not readable).
    pub unknowns: Vec<String>,
}
