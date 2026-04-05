//! Probe trait — domain contract for proprioceptive sensing.
//! The kernel senses its own infrastructure through typed, bounded probes.
//! Pure domain — zero external dependencies.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─── Status ──────────────────────────────────────────────────────────────────

/// Operational status reported by a probe.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Ok,
    Degraded,
    Unavailable,
    Denied,
}

// ─── Detail Structs ───────────────────────────────────────────────────────────

/// CPU, memory, disk, and load for a host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDetails {
    pub cpu_usage_percent: Option<f32>,
    pub memory_used_gb: Option<f64>,
    pub memory_total_gb: Option<f64>,
    pub disk_available_gb: Option<f64>,
    pub disk_total_gb: Option<f64>,
    pub load_average_1m: Option<f64>,
    pub uptime_seconds: Option<u64>,
}

/// Backup inventory for a host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupDetails {
    pub last_backup_age_hours: Option<f64>,
    pub last_backup_size_mb: Option<f64>,
    pub backup_count: Option<u32>,
    pub backup_dir: String,
}

/// Reachability and latency for a single Dog backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DogHealthDetails {
    pub dog_name: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    /// Actual context tokens supported by the running model (if available).
    pub actual_n_ctx: Option<u32>,
    /// Expected context tokens from configuration.
    pub expected_n_ctx: Option<u32>,
    /// Actual model name reported by /v1/models (if available).
    #[serde(default)]
    pub actual_model: Option<String>,
    /// Expected model name from backends.toml.
    #[serde(default)]
    pub expected_model: Option<String>,
    /// True when actual_model doesn't match expected_model.
    #[serde(default)]
    pub model_mismatch: bool,
}

/// A single network interface observed on a host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInterfaceInfo {
    pub name: String,
    pub state: String,
    pub ips: Vec<String>,
}

/// Network interfaces observed on a host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDetails {
    pub interfaces: Vec<NetInterfaceInfo>,
}

/// Self-monitoring: the kernel's own process metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDetails {
    /// Resident set size in megabytes (VmRSS).
    pub memory_rss_mb: Option<f64>,
    /// Number of open file descriptors.
    pub fd_count: Option<u64>,
    /// File descriptor limit (RLIMIT_NOFILE soft).
    pub fd_limit: Option<u64>,
    /// Number of threads.
    pub thread_count: Option<u64>,
    /// Cumulative user-mode CPU seconds (utime from /proc/self/stat).
    pub cpu_user_seconds: Option<f64>,
    /// Cumulative system-mode CPU seconds (stime from /proc/self/stat).
    pub cpu_system_seconds: Option<f64>,
}

/// Linux Pressure Stall Information (PSI) from /proc/pressure/*.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureDetails {
    pub cpu_some_avg10: Option<f64>,
    pub cpu_some_avg60: Option<f64>,
    pub memory_some_avg10: Option<f64>,
    pub memory_full_avg10: Option<f64>,
    pub io_some_avg10: Option<f64>,
    pub io_full_avg10: Option<f64>,
}

/// Fleet health: per-Dog backend reachability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetDetails {
    pub dogs: Vec<DogHealthDetails>,
}

// ─── Typed Details ────────────────────────────────────────────────────────────

/// Typed payload carried by a `ProbeResult`.
/// Each real probe uses a dedicated variant; `Empty` is reserved for test doubles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum ProbeDetails {
    Resource(ResourceDetails),
    Backup(BackupDetails),
    Network(NetworkDetails),
    Process(ProcessDetails),
    Pressure(PressureDetails),
    Fleet(FleetDetails),
    /// NullProbe and test doubles only. Real probes must use a typed variant.
    Empty,
}

// ─── ProbeResult ─────────────────────────────────────────────────────────────

/// The output of a single probe execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub name: String,
    pub status: ProbeStatus,
    pub details: ProbeDetails,
    pub duration_ms: u64,
    /// RFC3339 timestamp of when the probe completed.
    pub timestamp: String,
}

// ─── EnvironmentSnapshot ─────────────────────────────────────────────────────

/// A point-in-time snapshot of all probe results — the kernel's proprioceptive state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    /// RFC3339 timestamp of when the snapshot was assembled.
    pub timestamp: String,
    pub probes: Vec<ProbeResult>,
    pub overall: ProbeStatus,
}

impl EnvironmentSnapshot {
    /// Aggregate worst status across all probes.
    /// Severity order: Denied > Unavailable > Degraded > Ok.
    /// Returns `Unavailable` if `probes` is empty.
    pub fn worst_status(probes: &[ProbeResult]) -> ProbeStatus {
        if probes.is_empty() {
            return ProbeStatus::Unavailable;
        }
        let mut worst = ProbeStatus::Ok;
        for probe in probes {
            worst = match (worst, probe.status) {
                (_, ProbeStatus::Denied) | (ProbeStatus::Denied, _) => ProbeStatus::Denied,
                (_, ProbeStatus::Unavailable) | (ProbeStatus::Unavailable, _) => {
                    ProbeStatus::Unavailable
                }
                (_, ProbeStatus::Degraded) | (ProbeStatus::Degraded, _) => ProbeStatus::Degraded,
                (ProbeStatus::Ok, ProbeStatus::Ok) => ProbeStatus::Ok,
            };
            // Short-circuit: Denied is the worst possible status.
            if worst == ProbeStatus::Denied {
                break;
            }
        }
        worst
    }

    /// Check if probes are degraded from a shared RwLock.
    /// Returns true (degraded) on poison, missing snapshot, or non-Ok overall.
    /// K14: poison/missing = assume degraded, never optimistic.
    pub fn is_degraded(env: &std::sync::RwLock<Option<EnvironmentSnapshot>>) -> bool {
        match env.read() {
            Ok(guard) => guard
                .as_ref()
                .map(|s| s.overall != ProbeStatus::Ok)
                .unwrap_or(true), // no snapshot yet = degraded
            Err(_) => true, // poison = degraded
        }
    }
}

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("Probe internal error: {0}")]
    Internal(String),
}

// ─── Probe Trait ─────────────────────────────────────────────────────────────

/// Domain port for a single proprioceptive probe.
/// Each implementation senses one aspect of the kernel's operating environment.
#[async_trait]
pub trait Probe: Send + Sync {
    /// Human-readable name identifying this probe (e.g. "resource", "backup").
    fn name(&self) -> &str;

    /// How frequently this probe should be polled.
    fn interval(&self) -> Duration;

    /// Execute the probe and return a typed result.
    async fn sense(&self) -> Result<ProbeResult, ProbeError>;
}

// ─── NullProbe ────────────────────────────────────────────────────────────────

/// Test double — always reports Unavailable with no detail.
/// Use in unit tests or when no real probe is configured.
#[derive(Debug)]
pub struct NullProbe;

#[async_trait]
impl Probe for NullProbe {
    fn name(&self) -> &str {
        "null"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        Ok(ProbeResult {
            name: "null".to_string(),
            status: ProbeStatus::Unavailable,
            details: ProbeDetails::Empty,
            duration_ms: 0,
            timestamp: "1970-01-01T00:00:00Z".to_string(),
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_probe_returns_unavailable() {
        let probe = NullProbe;
        let result = probe.sense().await.expect("NullProbe must not error");
        assert_eq!(result.status, ProbeStatus::Unavailable);
    }

    #[test]
    fn worst_status_picks_worst() {
        // Ok < Degraded < Unavailable < Denied
        let make = |status: ProbeStatus| ProbeResult {
            name: "test".to_string(),
            status,
            details: ProbeDetails::Empty,
            duration_ms: 0,
            timestamp: "1970-01-01T00:00:00Z".to_string(),
        };

        // Single-element cases
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[make(ProbeStatus::Ok)]),
            ProbeStatus::Ok
        );
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[make(ProbeStatus::Degraded)]),
            ProbeStatus::Degraded
        );
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[make(ProbeStatus::Unavailable)]),
            ProbeStatus::Unavailable
        );
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[make(ProbeStatus::Denied)]),
            ProbeStatus::Denied
        );

        // Mixed: worst wins
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[
                make(ProbeStatus::Ok),
                make(ProbeStatus::Degraded)
            ]),
            ProbeStatus::Degraded
        );
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[
                make(ProbeStatus::Degraded),
                make(ProbeStatus::Unavailable)
            ]),
            ProbeStatus::Unavailable
        );
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[
                make(ProbeStatus::Ok),
                make(ProbeStatus::Denied),
                make(ProbeStatus::Degraded)
            ]),
            ProbeStatus::Denied
        );
    }

    #[test]
    fn empty_probes_returns_unavailable() {
        assert_eq!(
            EnvironmentSnapshot::worst_status(&[]),
            ProbeStatus::Unavailable
        );
    }
}
