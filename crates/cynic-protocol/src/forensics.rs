//! Forensics — post-crash evidence collection and post-mortem reporting.
//!
//! Types are pure (no I/O) so they can be tested without a machine and shared
//! across every layer: host adapters, kernel events, CLI projection, SOC.
//!
//! Calibrated on real crash evidence collected 2026-08-30 from the operator
//! host (boot_id `66515a0e…`). Every variant below maps to an observed signal.

use serde::{Deserialize, Serialize};

pub const FORENSIC_TIMELINE_SCHEMA: &str = "cynic.forensic_timeline.v1";
pub const POST_MORTEM_REPORT_SCHEMA: &str = "cynic.post_mortem_report.v1";

// ── Crash classification ─────────────────────────────────────────────────────

/// Kinds of host crash/freeze observed in the wild.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashKind {
    /// GPU page fault + ring timeout + reset failure. Observed 2026-08-30.
    GpuHang,
    /// Kernel traps (invalid opcode, softlockup). Observed 2026-08-18.
    SoftLockup,
    /// 203/EXEC explosion cascading from another crash. Observed constantly.
    ServiceCascade,
    /// Reboot with no trace (5 min uptime). Observed 2026-08-25.
    Immediate,
    /// dbus timeout, journald WATCHDOG failures, progressive degradation.
    ProgressiveFreeze,
    /// Unknown — evidence collected but classification inconclusive.
    Unknown,
}

impl CrashKind {
    /// Human-readable label for projection.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GpuHang => "gpu_hang",
            Self::SoftLockup => "soft_lockup",
            Self::ServiceCascade => "service_cascade",
            Self::Immediate => "immediate",
            Self::ProgressiveFreeze => "progressive_freeze",
            Self::Unknown => "unknown",
        }
    }
}

// ── Evidence sources ─────────────────────────────────────────────────────────

/// Where a piece of evidence came from. Each source has different access
/// requirements (user vs root) and retention characteristics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum EvidenceSource {
    /// `journalctl --boot <id>` — user-accessible, persistent across reboots.
    Journalctl { boot_id: String, timestamp: i64 },
    /// kernel ring buffer — may be empty for non-root (rate-limited/filtered).
    Dmesg { timestamp: String },
    /// Apport crash dump under /var/crash — user-readable for same-uid dumps.
    CoreDump {
        path: String,
        size: u64,
        executable: String,
        mtime: i64,
    },
    /// GPU dev coredump (/sys/class/drm/.../devcoredump/data) — root only.
    DevCoredump { device: String, path: String },
    /// `journalctl --list-boots` — boot table entries.
    BootTable {
        boot_id: String,
        first_entry: i64,
        last_entry: i64,
    },
    /// cynic-kernel PatternAnalyzer SSE stream disconnection — the canary.
    CynicCanary { disconnected_at: i64 },
    /// systemd unit failure — `systemctl --failed` / journal.
    SystemdFailed { unit: String, timestamp: i64 },
}

// ── Core evidence type ───────────────────────────────────────────────────────

/// One piece of evidence contributing to a crash/freeze investigation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForensicEvidence {
    pub source: EvidenceSource,
    pub kind: CrashKind,
    /// Raw snippet extracted from the source (log line, dump header, etc.)
    pub snippet: String,
    /// 0-100, higher = more severe. Calibrated on observed crashes.
    pub severity: u8,
}

// ── Service loop ─────────────────────────────────────────────────────────────

/// A systemd unit stuck in a restart loop — the most common crash signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceLoop {
    pub unit: String,
    pub restart_count: u64,
    /// systemd exit status: 203 = EXEC, 1 = FAILURE, 2 = INVALIDARG.
    pub exit_status: u8,
    /// Window over which restarts were counted (seconds).
    pub window_secs: u64,
}

impl ServiceLoop {
    /// Restart rate per hour.
    pub fn rate_per_hour(&self) -> f64 {
        if self.window_secs == 0 {
            return 0.0;
        }
        (self.restart_count as f64 / self.window_secs as f64) * 3600.0
    }
}

// ── Timeline ─────────────────────────────────────────────────────────────────

/// One entry in a forensic timeline — ordered by offset from boot start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub boot_id: String,
    /// Seconds since boot start (from /proc/uptime at evidence time).
    pub offset_secs: f64,
    pub evidence: ForensicEvidence,
}

/// A complete forensic timeline for one boot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForensicTimeline {
    pub schema: String,
    pub boot_id: String,
    pub entries: Vec<TimelineEntry>,
    pub generated_at: i64,
}

// ── Post-mortem report ───────────────────────────────────────────────────────

/// The final output: everything known about a crash, assembled from all sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostMortemReport {
    #[serde(default = "default_post_mortem_schema")]
    pub schema: String,
    /// The boot that crashed.
    pub boot_id: String,
    /// Previous boot (for chaining).
    pub previous_boot_id: Option<String>,
    /// Uptime before crash (None if unknown).
    pub uptime_before_crash_secs: Option<u64>,
    /// Classified crash kind (None if inconclusive).
    pub crash: Option<CrashKind>,
    /// Count of GPU page faults / resets observed.
    pub gpu_fault_count: u32,
    /// Count of kernel traps (invalid opcode, softlockup).
    pub softlockup_trap_count: u32,
    /// Service restart loops active during the crash window.
    pub service_loops: Vec<ServiceLoop>,
    /// Number of cynic-kernel SSE stream disconnections.
    pub cynic_canary_disconnections: u32,
    /// Paths to readable Apport crash dumps.
    pub core_dumps: Vec<String>,
    /// Paths to GPU dev coredumps (root-only, may be empty for user).
    pub dev_coredumps: Vec<String>,
    /// When this report was generated.
    pub generated_at: i64,
}

fn default_post_mortem_schema() -> String {
    POST_MORTEM_REPORT_SCHEMA.to_string()
}
// ── SOC thresholds ───────────────────────────────────────────────────────────

/// Detection thresholds calibrated on real crash evidence.
/// Each threshold maps to an observed signal from 2026-08-18 / 2026-08-30.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SocThresholds {
    /// amdgpu page faults in 60s window → GpuHang suspicion.
    pub gpu_fault_window_secs: u64,
    pub gpu_fault_count: u32,
    /// Kernel traps in 60s window → SoftLockup.
    pub kernel_trap_window_secs: u64,
    pub kernel_trap_count: u32,
    /// cynic-kernel SSE disconnections in 300s → freeze.
    pub canary_disconnect_window_secs: u64,
    pub canary_disconnect_count: u32,
    /// journald WATCHDOG failures → transport broken.
    pub journald_watchdog_count: u32,
    /// 203/EXEC delta per hour → service cascade.
    pub service_exec_delta_per_hour: u64,
    /// Service restart rate per hour → crash-loop.
    pub service_restart_rate_per_hour: f64,
}

impl Default for SocThresholds {
    fn default() -> Self {
        Self {
            gpu_fault_window_secs: 60,
            gpu_fault_count: 1,
            kernel_trap_window_secs: 60,
            kernel_trap_count: 2,
            canary_disconnect_window_secs: 300,
            canary_disconnect_count: 3,
            journald_watchdog_count: 5,
            service_exec_delta_per_hour: 1000,
            service_restart_rate_per_hour: 500.0,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_kind_roundtrips() {
        for kind in [
            CrashKind::GpuHang,
            CrashKind::SoftLockup,
            CrashKind::ServiceCascade,
            CrashKind::Immediate,
            CrashKind::ProgressiveFreeze,
            CrashKind::Unknown,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let decoded: CrashKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, decoded);
        }
    }

    #[test]
    fn crash_kind_labels() {
        assert_eq!(CrashKind::GpuHang.as_str(), "gpu_hang");
        assert_eq!(CrashKind::SoftLockup.as_str(), "soft_lockup");
        assert_eq!(CrashKind::Unknown.as_str(), "unknown");
    }

    #[test]
    fn evidence_source_roundtrips() {
        let source = EvidenceSource::Journalctl {
            boot_id: "abc123".to_string(),
            timestamp: 1724000000,
        };
        let json = serde_json::to_string(&source).unwrap();
        let decoded: EvidenceSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, decoded);
    }

    #[test]
    fn forensic_evidence_serializes() {
        let evidence = ForensicEvidence {
            source: EvidenceSource::Dmesg {
                timestamp: "2026-08-30T22:35:12".to_string(),
            },
            kind: CrashKind::GpuHang,
            snippet: "amdgpu 0000:30:00.0: ring gfx timeout".into(),
            severity: 80,
        };
        let json = serde_json::to_string(&evidence).unwrap();
        assert!(json.contains("gpu_hang"));
        assert!(json.contains("ring gfx timeout"));
    }

    #[test]
    fn service_loop_rate() {
        let sl = ServiceLoop {
            unit: "discord-pac-server".into(),
            restart_count: 64034,
            exit_status: 203,
            window_secs: 86400,
        };
        let rate = sl.rate_per_hour();
        assert!(rate > 0.0);
        assert!(rate < 3000.0); // ~2668/h for 64034/day
    }

    #[test]
    fn post_mortem_report_serializes() {
        let report = PostMortemReport {
            schema: POST_MORTEM_REPORT_SCHEMA.to_string(),
            boot_id: "66515a0e".into(),
            previous_boot_id: Some("1ff29dfc".into()),
            uptime_before_crash_secs: Some(43140),
            crash: Some(CrashKind::GpuHang),
            gpu_fault_count: 12,
            softlockup_trap_count: 0,
            service_loops: vec![ServiceLoop {
                unit: "discord-pac-server".into(),
                restart_count: 8454,
                exit_status: 203,
                window_secs: 3600,
            }],
            cynic_canary_disconnections: 0,
            core_dumps: vec!["/var/crash/_usr_bin_bash.1000.crash".into()],
            dev_coredumps: vec![],
            generated_at: 1724000000,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("gpu_hang"));
        assert!(json.contains(POST_MORTEM_REPORT_SCHEMA));
    }

    #[test]
    fn thresholds_default_calibrated() {
        let t = SocThresholds::default();
        assert_eq!(t.gpu_fault_count, 1);
        assert_eq!(t.kernel_trap_count, 2);
        assert_eq!(t.canary_disconnect_count, 3);
        assert_eq!(t.journald_watchdog_count, 5);
        assert_eq!(t.service_exec_delta_per_hour, 1000);
        assert_eq!(t.service_restart_rate_per_hour, 500.0);
    }

    #[test]
    fn schema_is_stable() {
        assert_eq!(FORENSIC_TIMELINE_SCHEMA, "cynic.forensic_timeline.v1");
        assert_eq!(POST_MORTEM_REPORT_SCHEMA, "cynic.post_mortem_report.v1");
    }
}
