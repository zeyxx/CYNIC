//! SOC detection — pure functions that check forensics signals against thresholds.
//!
//! Canonical owner: `cynic-intel` (SoC foundation §481).
//! Calibrated on real crash evidence from 2026-08-18 and 2026-08-30.
//! Every function takes observed counts and returns an Alert when threshold exceeded.

use super::types::Alert;
/// Thresholds for SOC crash-pattern detection.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SocThresholds {
    pub gpu_fault_window_secs: u64,
    pub gpu_fault_count: u32,
    pub kernel_trap_window_secs: u64,
    pub kernel_trap_count: u32,
    pub canary_disconnect_window_secs: u64,
    pub canary_disconnect_count: u32,
    pub watchdog_failure_count: u32,
    pub service_cascade_exec_delta: u64,
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
            watchdog_failure_count: 5,
            service_cascade_exec_delta: 1000,
            service_restart_rate_per_hour: 500.0,
        }
    }
}

/// Snapshot of forensics counters for one detection tick.
#[derive(Debug, Clone, Default)]
pub struct ForensicsSnapshot {
    pub boot_id: String,
    pub gpu_faults: u32,
    pub kernel_traps: u32,
    pub service_cascades: u64,
    pub canary_disconnects: u32,
    pub watchdog_failures: u32,
    pub service_restart_rate: f64,
    pub dbus_timeouts: u32,
}

/// GPU page fault detection. ≥ 1 fault in 60s window → GpuHang suspicion.
pub fn detect_gpu_fault(thresholds: &SocThresholds, fault_count: u32) -> Option<Alert> {
    if fault_count >= thresholds.gpu_fault_count {
        Some(Alert {
            kind: "soc_gpu_hang",
            message: format!(
                "GPU page fault detected ({fault_count} faults in window, threshold: {}). \
                 amdgpu ring timeout / ASIC reset imminent. GpuHang risk.",
                thresholds.gpu_fault_count
            ),
            severity: "critical",
        })
    } else {
        None
    }
}

/// Kernel trap detection. ≥ 2 traps in 60s → SoftLockup.
pub fn detect_softlockup(thresholds: &SocThresholds, trap_count: u32) -> Option<Alert> {
    if trap_count >= thresholds.kernel_trap_count {
        Some(Alert {
            kind: "soc_softlockup",
            message: format!(
                "Kernel trap burst detected ({trap_count} traps in window, threshold: {}). \
                 MCE / RCU stall / DWARF recursion pattern. SoftLockup risk.",
                thresholds.kernel_trap_count
            ),
            severity: "critical",
        })
    } else {
        None
    }
}

/// Service cascade detection. ≥ 1000 203/EXEC per hour → cascade.
pub fn detect_service_cascade(thresholds: &SocThresholds, exec_delta: u64) -> Option<Alert> {
    if exec_delta >= thresholds.service_cascade_exec_delta {
        Some(Alert {
            kind: "soc_service_cascade",
            message: format!(
                "Service failure cascade detected ({exec_delta} 203/EXEC in 1h, threshold: {}). \
                 systemd cgroup exhaustion / journald DOS imminent.",
                thresholds.service_cascade_exec_delta
            ),
            severity: "critical",
        })
    } else {
        None
    }
}

/// Cynic canary disconnect detection. ≥ 3 SSE disconnects in 300s → freeze.
pub fn detect_canary_disconnect(
    thresholds: &SocThresholds,
    disconnect_count: u32,
) -> Option<Alert> {
    if disconnect_count >= thresholds.canary_disconnect_count {
        Some(Alert {
            kind: "soc_canary_disconnect",
            message: format!(
                "Canary agent disconnects ({disconnect_count} drops in 300s, threshold: {}). \
                 tokio runtime stall or network partition.",
                thresholds.canary_disconnect_count
            ),
            severity: "warning",
        })
    } else {
        None
    }
}

/// journald WATCHDOG failure detection. ≥ 5 → transport broken.
pub fn detect_journald_watchdog(thresholds: &SocThresholds, failure_count: u32) -> Option<Alert> {
    if failure_count >= thresholds.watchdog_failure_count {
        Some(Alert {
            kind: "soc_journald_watchdog",
            message: format!(
                "journald WATCHDOG failures ({failure_count} in window, threshold: {}). \
                 systemd transport /dev/log saturated or blocked.",
                thresholds.watchdog_failure_count
            ),
            severity: "warning",
        })
    } else {
        None
    }
}

/// Service restart rate detection. ≥ 500 restarts/hour → crash-loop.
pub fn detect_service_restart_rate(
    thresholds: &SocThresholds,
    rate_per_hour: f64,
) -> Option<Alert> {
    if rate_per_hour >= thresholds.service_restart_rate_per_hour {
        Some(Alert {
            kind: "soc_crash_loop",
            message: format!(
                "High service restart rate ({rate_per_hour:.0}/hour, threshold: {:.0}). \
                 CrashLoopBackOff pattern detected.",
                thresholds.service_restart_rate_per_hour
            ),
            severity: "warning",
        })
    } else {
        None
    }
}

/// Progressive freeze detection — dbus timeout pattern.
pub fn detect_dbus_timeout(count: u32) -> Option<Alert> {
    if count >= 3 {
        Some(Alert {
            kind: "soc_dbus_timeout",
            message: format!(
                "D-Bus timeouts detected ({count} occurrences). \
                 ProgressiveFreeze pattern: systemd PID 1 may be non-responsive."
            ),
            severity: "critical",
        })
    } else {
        None
    }
}

/// Combined check: all detection functions applied to one forensics snapshot.
pub fn check_all(thresholds: &SocThresholds, snapshot: &ForensicsSnapshot) -> Vec<Alert> {
    let mut alerts = Vec::new();
    if let Some(a) = detect_gpu_fault(thresholds, snapshot.gpu_faults) {
        alerts.push(a);
    }
    if let Some(a) = detect_softlockup(thresholds, snapshot.kernel_traps) {
        alerts.push(a);
    }
    if let Some(a) = detect_service_cascade(thresholds, snapshot.service_cascades) {
        alerts.push(a);
    }
    if let Some(a) = detect_canary_disconnect(thresholds, snapshot.canary_disconnects) {
        alerts.push(a);
    }
    if let Some(a) = detect_journald_watchdog(thresholds, snapshot.watchdog_failures) {
        alerts.push(a);
    }
    if let Some(a) = detect_service_restart_rate(thresholds, snapshot.service_restart_rate) {
        alerts.push(a);
    }
    if let Some(a) = detect_dbus_timeout(snapshot.dbus_timeouts) {
        alerts.push(a);
    }
    alerts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_fault_detection() {
        let t = SocThresholds::default();
        assert!(detect_gpu_fault(&t, 0).is_none());
        let a = detect_gpu_fault(&t, 1).expect("alert on 1 fault");
        assert_eq!(a.kind, "soc_gpu_hang");
        assert_eq!(a.severity, "critical");
    }

    #[test]
    fn test_softlockup_detection() {
        let t = SocThresholds::default();
        assert!(detect_softlockup(&t, 1).is_none());
        let a = detect_softlockup(&t, 2).expect("alert on 2 traps");
        assert_eq!(a.kind, "soc_softlockup");
        assert_eq!(a.severity, "critical");
    }

    #[test]
    fn test_service_cascade_detection() {
        let t = SocThresholds::default();
        assert!(detect_service_cascade(&t, 500).is_none());
        let a = detect_service_cascade(&t, 1000).expect("alert on 1000 exec delta");
        assert_eq!(a.kind, "soc_service_cascade");
        assert_eq!(a.severity, "critical");
    }

    #[test]
    fn test_canary_disconnect_detection() {
        let t = SocThresholds::default();
        assert!(detect_canary_disconnect(&t, 2).is_none());
        let a = detect_canary_disconnect(&t, 3).expect("alert on 3 disconnects");
        assert_eq!(a.kind, "soc_canary_disconnect");
        assert_eq!(a.severity, "warning");
    }

    #[test]
    fn test_check_all_clean_snapshot() {
        let t = SocThresholds::default();
        let snap = ForensicsSnapshot::default();
        let alerts = check_all(&t, &snap);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_check_all_multiple_failures() {
        let t = SocThresholds::default();
        let snap = ForensicsSnapshot {
            boot_id: "test-boot".to_string(),
            gpu_faults: 2,
            kernel_traps: 5,
            service_cascades: 1500,
            canary_disconnects: 4,
            watchdog_failures: 6,
            service_restart_rate: 600.0,
            dbus_timeouts: 3,
        };
        let alerts = check_all(&t, &snap);
        assert_eq!(alerts.len(), 7);
    }
}
