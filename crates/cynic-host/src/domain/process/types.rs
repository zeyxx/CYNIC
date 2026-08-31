//! Pure types for process-level health: crash-loop detection.
//!
//! The crash-loop detector observes a sequence of process (re)start events
//! with monotonic timestamps and classifies a service as being in
//! `CrashLoopBackoff` when restarts exceed a configurable threshold within
//! a sliding window. Unknown is data: a service with zero observations is
//! `Unobservable`, never `Healthy`.

use serde::{Deserialize, Serialize};

/// Schema identifier for crash-loop reports emitted to cynic.fleet_*.v1
pub const CRASH_LOOP_REPORT_SCHEMA: &str = "cynic.host_process_crashloop.v1";

/// A single observed (re)start event.
///
/// `timestamp_unix_s` is a monotonic clock reading (seconds since epoch),
/// guaranteed strictly non-decreasing by the adapter before this type is
/// produced. The detector trusts that ordering invariant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartEvent {
    pub timestamp_unix_s: u64,
}

/// Health level of a service process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessHealth {
    /// Running normally; no crash-loop detected.
    Healthy,
    /// In rapid restart cycle — exceeds restart threshold within the window.
    CrashLoopBackoff,
    /// Probe could not observe; status is unknown, never fabricated.
    Unobservable,
}

/// Detector parameters (pure policy, injected by caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashLoopParams {
    /// How many restarts within `window_seconds` trigger CrashLoopBackoff.
    pub restart_threshold: u32,
    /// Sliding window in seconds for the restart count.
    pub window_seconds: u64,
}

impl Default for CrashLoopParams {
    /// Industry-standard crash-loop threshold:
    /// > 3 restarts within 5 minutes → backoff (matches systemd RestartPreventExitStatus
    /// / Kubernetes default CrashLoopBackOff semantics).
    fn default() -> Self {
        Self {
            restart_threshold: 3,
            window_seconds: 300,
        }
    }
}

/// Output of the crash-loop probe for one service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashLoopStatus {
    pub schema: String,
    pub service_name: String,
    /// How many restart events were observed in the window.
    pub restarts_in_window: u32,
    pub level: ProcessHealth,
    /// Window parameters used for this classification.
    pub params: CrashLoopParams,
    /// Human-readable reason when classified as CrashLoopBackoff.
    pub reason: Option<String>,
}

/// Pure crash-loop classification.
///
/// Given a sorted sequence of restart timestamps and detector params, returns
/// the classified status. No I/O.
///
/// # Arguments
/// * `service_name` — the systemd unit or process name being classified
/// * `events` — restart events, MUST be sorted ascending by `timestamp_unix_s`
/// * `now_unix_s` — current monotonic time to anchor the sliding window
/// * `params` — detector thresholds
///
/// # Returns
/// `CrashLoopStatus` with:
/// - `ProcessHealth::CrashLoopBackoff` if `restarts_in_window > restart_threshold`
/// - `ProcessHealth::Unobservable` if `events` is empty (no data)
/// - `ProcessHealth::Healthy` otherwise
pub fn classify_crashloop(
    service_name: &str,
    events: &[RestartEvent],
    now_unix_s: u64,
    params: CrashLoopParams,
) -> CrashLoopStatus {
    // Empty events → unknown, never healthy (unknown is data)
    if events.is_empty() {
        return CrashLoopStatus {
            schema: CRASH_LOOP_REPORT_SCHEMA.to_string(),
            service_name: service_name.to_string(),
            restarts_in_window: 0,
            level: ProcessHealth::Unobservable,
            params,
            reason: Some("no_restart_events_observed".to_string()),
        };
    }

    // Sliding window: count restarts within [now - window, now]
    let window_start = now_unix_s.saturating_sub(params.window_seconds);
    let restarts_in_window = events
        .iter()
        .filter(|e| e.timestamp_unix_s >= window_start && e.timestamp_unix_s <= now_unix_s)
        .count() as u32;

    let level = if restarts_in_window > params.restart_threshold {
        ProcessHealth::CrashLoopBackoff
    } else {
        ProcessHealth::Healthy
    };

    CrashLoopStatus {
        schema: CRASH_LOOP_REPORT_SCHEMA.to_string(),
        service_name: service_name.to_string(),
        restarts_in_window,
        level,
        params,
        reason: if level == ProcessHealth::CrashLoopBackoff {
            Some(format!(
                "restarts exceed threshold {} within {}s window",
                params.restart_threshold, params.window_seconds
            ))
        } else {
            None
        },
    }
}

/// Project a CrashLoopStatus into a HealthSignalPayload.
///
/// Returns `None` for anything that is NOT a crash-loop — healthy and
/// unobservable states emit no anomaly.
pub fn crashloop_to_health_signal(
    status: &CrashLoopStatus,
) -> Option<cynic_protocol::health_signal::HealthSignalPayload> {
    match status.level {
        ProcessHealth::CrashLoopBackoff => Some(
            cynic_protocol::health_signal::HealthSignalPayload::InferenceCrashLoop {
                service: status.service_name.clone(),
                failure_rate_percent: 100.0, // CrashLoop == total failure rate within window
                consecutive_failures: status.restarts_in_window,
            },
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: events at given second-marks
    fn events_at(times: &[u64]) -> Vec<RestartEvent> {
        times
            .iter()
            .map(|t| RestartEvent {
                timestamp_unix_s: *t,
            })
            .collect()
    }

    #[test]
    fn healthy_when_under_threshold() {
        let events = events_at(&[10, 20, 30]);
        let status = classify_crashloop("cynic-node", &events, 35, CrashLoopParams::default());
        assert_eq!(status.level, ProcessHealth::Healthy);
        assert_eq!(status.restarts_in_window, 3);
        assert_eq!(status.reason, None);
    }

    #[test]
    fn crashloop_when_over_threshold() {
        let events = events_at(&[10, 20, 30, 40, 50]);
        let status = classify_crashloop("cynic-node", &events, 60, CrashLoopParams::default());
        assert_eq!(status.level, ProcessHealth::CrashLoopBackoff);
        assert_eq!(status.restarts_in_window, 4);
        assert!(status.reason.is_some());
    }

    #[test]
    fn unobservable_when_no_events() {
        let status = classify_crashloop("cynic-node", &[], 100, CrashLoopParams::default());
        assert_eq!(status.level, ProcessHealth::Unobservable);
        assert_eq!(status.restarts_in_window, 0);
    }

    #[test]
    fn sliding_window_excludes_old_events() {
        // Window = [130, 430] (300s back from now=430)
        // Events: 100 (excluded), 110 (excluded), 400 (in), 410 (in), 420 (in) → 3 in window
        let events = events_at(&[100, 110, 400, 410, 420]);
        let status = classify_crashloop("svc", &events, 430, CrashLoopParams::default());
        assert_eq!(status.restarts_in_window, 3);
        // 3 <= threshold(3) → healthy, NOT crashloop
        assert_eq!(status.level, ProcessHealth::Healthy);
    }

    #[test]
    fn sliding_window_crashes_only_recent() {
        // Window = [130, 430]; events: 100,110 (old, excluded) + 400,410,420,425 (4 in window)
        let events = events_at(&[100, 110, 400, 410, 420, 425]);
        let status = classify_crashloop("svc", &events, 430, CrashLoopParams::default());
        assert_eq!(status.restarts_in_window, 4);
        assert_eq!(status.level, ProcessHealth::CrashLoopBackoff);
    }

    #[test]
    fn custom_params() {
        let events = events_at(&[0, 10, 20, 30]);
        let params = CrashLoopParams {
            restart_threshold: 10,
            window_seconds: 60,
        };
        let status = classify_crashloop("svc", &events, 40, params);
        assert_eq!(status.level, ProcessHealth::Healthy);
    }

    #[test]
    fn crashloop_threshold_boundary_is_strict_gt() {
        // Exactly threshold (3) restarts → healthy (not >)
        let events = events_at(&[10, 20, 30]);
        let status = classify_crashloop("svc", &events, 35, CrashLoopParams::default());
        assert_eq!(status.level, ProcessHealth::Healthy);

        // One more → crashloop
        let events = events_at(&[10, 20, 30, 31]);
        let status = classify_crashloop("svc", &events, 35, CrashLoopParams::default());
        assert_eq!(status.level, ProcessHealth::CrashLoopBackoff);
    }

    #[test]
    fn crashloop_projects_to_health_signal() {
        let status = CrashLoopStatus {
            schema: CRASH_LOOP_REPORT_SCHEMA.to_string(),
            service_name: "cynic-node".to_string(),
            restarts_in_window: 5,
            level: ProcessHealth::CrashLoopBackoff,
            params: CrashLoopParams::default(),
            reason: Some("over threshold".to_string()),
        };
        let signal = crashloop_to_health_signal(&status);
        assert!(signal.is_some());
    }

    #[test]
    fn healthy_projects_no_signal() {
        let status = CrashLoopStatus {
            schema: CRASH_LOOP_REPORT_SCHEMA.to_string(),
            service_name: "cynic-node".to_string(),
            restarts_in_window: 2,
            level: ProcessHealth::Healthy,
            params: CrashLoopParams::default(),
            reason: None,
        };
        let signal = crashloop_to_health_signal(&status);
        assert!(signal.is_none());
    }
}
