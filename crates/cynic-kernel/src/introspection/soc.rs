//! SOC detection — pure functions that check forensics signals against thresholds.
//!
//! Calibrated on real crash evidence from 2026-08-18 and 2026-08-30.
//! Every function takes observed counts and returns an Alert when threshold exceeded.

use crate::introspection::types::Alert;
pub use cynic_protocol::forensics::SocThresholds;

/// GPU page fault detection. ≥ 1 fault in 60s window → GpuHang suspicion.
pub fn detect_gpu_fault(thresholds: &SocThresholds, fault_count: u32) -> Option<Alert> {
    if fault_count >= thresholds.gpu_fault_count {
        Some(Alert {
            kind: "gpu_fault",
            message: format!(
                "GPU page fault detected: {fault_count} in {}s window — possible GpuHang",
                thresholds.gpu_fault_window_secs
            ),
            severity: "warning",
        })
    } else {
        None
    }
}

/// Kernel trap detection. ≥ 2 traps in 60s → SoftLockup.
pub fn detect_softlockup(thresholds: &SocThresholds, trap_count: u32) -> Option<Alert> {
    if trap_count >= thresholds.kernel_trap_count {
        Some(Alert {
            kind: "softlockup",
            message: format!(
                "Kernel traps detected: {trap_count} in {}s window — possible softlockup",
                thresholds.kernel_trap_window_secs
            ),
            severity: "critical",
        })
    } else {
        None
    }
}

/// Service cascade detection. ≥ 1000 203/EXEC per hour → cascade.
pub fn detect_service_cascade(thresholds: &SocThresholds, exec_delta: u64) -> Option<Alert> {
    if exec_delta >= thresholds.service_exec_delta_per_hour {
        Some(Alert {
            kind: "service_cascade",
            message: format!(
                "Service cascade: {exec_delta} 203/EXEC in 1h — check host execution environment"
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
            kind: "canary_disconnect",
            message: format!(
                "Cynic canary: {disconnect_count} SSE disconnects in {}s — host may be freezing",
                thresholds.canary_disconnect_window_secs
            ),
            severity: "warning",
        })
    } else {
        None
    }
}

/// journald WATCHDOG failure detection. ≥ 5 → transport broken.
pub fn detect_journald_watchdog(thresholds: &SocThresholds, failure_count: u32) -> Option<Alert> {
    if failure_count >= thresholds.journald_watchdog_count {
        Some(Alert {
            kind: "journald_watchdog",
            message: format!(
                "journald WATCHDOG: {failure_count} failures — transport endpoint may be broken"
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
            kind: "service_crash_loop",
            message: format!(
                "Service crash-loop: {rate_per_hour:.0} restarts/hour — exceeds threshold {}",
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
            kind: "progressive_freeze",
            message: format!("D-Bus timeouts: {count} — progressive freeze detected"),
            severity: "warning",
        })
    } else {
        None
    }
}

/// Combined check: all detection functions applied to one forensics snapshot.
pub fn check_all(thresholds: &SocThresholds, snapshot: &ForensicsSnapshot) -> Vec<Alert> {
    let mut alerts = Vec::new();
    if let Some(a) = detect_gpu_fault(thresholds, snapshot.gpu_fault_count) {
        alerts.push(a);
    }
    if let Some(a) = detect_softlockup(thresholds, snapshot.kernel_trap_count) {
        alerts.push(a);
    }
    if let Some(a) = detect_service_cascade(thresholds, snapshot.service_exec_delta) {
        alerts.push(a);
    }
    if let Some(a) = detect_canary_disconnect(thresholds, snapshot.canary_disconnect_count) {
        alerts.push(a);
    }
    if let Some(a) = detect_journald_watchdog(thresholds, snapshot.journald_watchdog_count) {
        alerts.push(a);
    }
    if let Some(a) = detect_service_restart_rate(thresholds, snapshot.service_restart_rate) {
        alerts.push(a);
    }
    if let Some(a) = detect_dbus_timeout(snapshot.dbus_timeout_count) {
        alerts.push(a);
    }
    alerts
}

/// Snapshot of forensics counters for one detection tick.
#[derive(Debug, Clone, Default)]
pub struct ForensicsSnapshot {
    pub gpu_fault_count: u32,
    pub kernel_trap_count: u32,
    pub service_exec_delta: u64,
    pub canary_disconnect_count: u32,
    pub journald_watchdog_count: u32,
    pub service_restart_rate: f64,
    pub dbus_timeout_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thresholds() -> SocThresholds {
        SocThresholds::default()
    }

    #[test]
    fn test_detect_gpu_fault_triggers_above_threshold() {
        let t = thresholds();
        let alert = detect_gpu_fault(&t, 1);
        assert!(alert.is_some());
        let a = alert.unwrap();
        assert_eq!(a.kind, "gpu_fault");
        assert_eq!(a.severity, "warning");
    }

    #[test]
    fn test_detect_gpu_fault_silent_below_threshold() {
        let t = thresholds();
        let alert = detect_gpu_fault(&t, 0);
        assert!(alert.is_none());
    }

    #[test]
    fn test_detect_softlockup_triggers() {
        let t = thresholds();
        let alert = detect_softlockup(&t, 2);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().severity, "critical");
    }

    #[test]
    fn test_detect_softlockup_silent() {
        let t = thresholds();
        assert!(detect_softlockup(&t, 1).is_none());
    }

    #[test]
    fn test_detect_service_cascade_triggers() {
        let t = thresholds();
        let alert = detect_service_cascade(&t, 1000);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().severity, "critical");
    }

    #[test]
    fn test_detect_service_cascade_silent() {
        let t = thresholds();
        assert!(detect_service_cascade(&t, 999).is_none());
    }

    #[test]
    fn test_detect_canary_disconnect_triggers() {
        let t = thresholds();
        let alert = detect_canary_disconnect(&t, 3);
        assert!(alert.is_some());
    }

    #[test]
    fn test_detect_canary_disconnect_silent() {
        let t = thresholds();
        assert!(detect_canary_disconnect(&t, 2).is_none());
    }

    #[test]
    fn test_detect_journald_watchdog_triggers() {
        let t = thresholds();
        let alert = detect_journald_watchdog(&t, 5);
        assert!(alert.is_some());
    }

    #[test]
    fn test_detect_journald_watchdog_silent() {
        let t = thresholds();
        assert!(detect_journald_watchdog(&t, 4).is_none());
    }

    #[test]
    fn test_detect_service_restart_rate_triggers() {
        let t = thresholds();
        let alert = detect_service_restart_rate(&t, 500.0);
        assert!(alert.is_some());
    }

    #[test]
    fn test_detect_service_restart_rate_silent() {
        let t = thresholds();
        assert!(detect_service_restart_rate(&t, 499.0).is_none());
    }

    #[test]
    fn test_detect_dbus_timeout_triggers() {
        let alert = detect_dbus_timeout(3);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().kind, "progressive_freeze");
    }

    #[test]
    fn test_detect_dbus_timeout_silent() {
        assert!(detect_dbus_timeout(2).is_none());
    }

    #[test]
    fn test_check_all_empty() {
        let t = thresholds();
        let snap = ForensicsSnapshot::default();
        let alerts = check_all(&t, &snap);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_check_all_multiple() {
        let t = thresholds();
        let snap = ForensicsSnapshot {
            gpu_fault_count: 3,
            kernel_trap_count: 2,
            service_exec_delta: 1500,
            canary_disconnect_count: 4,
            journald_watchdog_count: 6,
            service_restart_rate: 600.0,
            dbus_timeout_count: 5,
        };
        let alerts = check_all(&t, &snap);
        assert_eq!(alerts.len(), 7);
    }
}
