//! OS substrate health observation — the layer CYNIC was blind to during the
//! 2026-08-18 soft-lockup crash.
//!
//! The kernel observed dogs, services, contracts and substrate roles, but never
//! the host OS signals that actually froze the machine: kernel soft lockups that
//! did NOT panic (softlockup_panic=0), aggressive THP defrag under memory
//! pressure, and reclaim/OOM pressure. This module closes that gap by observing
//! those signals through portable files (no required root, no hardcoding) and
//! surfacing them as explicit, machine-readable anomalies.
//!
//! Split per structural-invariant #1: types (pure shapes), observe (read-only
//! portable probes), decision (pure policy), render (CLI projection only).
//! CLI surfaces must stay thin and project from `assess_os_health`.

pub mod decision;
pub mod observe;
pub mod render;
pub mod types;

pub use observe::{LocalOsHealthObserver, OsHealthObserver};
pub use types::OsObservation;

pub use decision::assessment_error_codes;
pub use decision::assessment_to_anomalies;
pub use decision::evaluate_os_health;

use self::types::{OsHealthAssessment, OsHealthConfig};

/// Runtime authority for OsHealth: default thresholds that catch the crash
/// class observed on 2026-08-18 (soft lockup with no panic, THP defrag,
/// memory reclaim pressure). Overridable via `OsHealthConfig`.
pub fn default_os_health_config() -> OsHealthConfig {
    OsHealthConfig {
        softlockup_panic_required: true,
        thp_defrag_forbid: vec![
            "always".to_string(),
            "defer".to_string(),
            "defer+madvise".to_string(),
        ],
        thp_enabled_forbid: vec!["always".to_string()],
        available_mem_mib_floor: 1024,
        reclaim_pressure_pct_ceiling: 60,
    }
}

/// Pure observe → decide. Portable: reads sysfs/proc via the injected
/// `OsHealthObserver` adapter so unit tests can feed fixed observations.
pub fn assess_os_health(
    config: &OsHealthConfig,
    observer: &dyn OsHealthObserver,
) -> OsHealthAssessment {
    let observed = observer.observe_os_health();
    decision::evaluate_os_health(config, &observed)
}

/// Helper used by CLI surfaces to assess with default authority config.
pub fn assess_os_health_default(observer: &dyn OsHealthObserver) -> OsHealthAssessment {
    assess_os_health(&default_os_health_config(), observer)
}

/// Stable schema tag for machine readers.
pub const OS_HEALTH_SCHEMA: &str = "cynic.os_health.assessment.v1";

/// Human projection for `cynic host os-health`.
pub fn os_health_human(assessment: &OsHealthAssessment) -> String {
    render::render_os_health_human(assessment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os_health::observe::StubOsHealthObserver;

    #[test]
    fn default_config_forbids_thp_always_and_requires_panic() {
        let cfg = default_os_health_config();
        assert!(cfg.softlockup_panic_required);
        assert!(cfg.thp_enabled_forbid.contains(&"always".to_string()));
        assert!(cfg.thp_defrag_forbid.contains(&"defer+madvise".to_string()));
    }

    #[test]
    fn healthy_substrate_assesses_clean() {
        let obs = super::types::OsObservation {
            softlockup_panic: true,
            thp_enabled: "madvise".to_string(),
            thp_defrag: "madvise".to_string(),
            available_mem_mib: 16384,
            reclaim_pressure_pct: 5,
            dmesg_softlockup_count: 0,
            dmesg_hung_task_count: 0,
            dmesg_oom_count: 0,
        };
        let a = assess_os_health_default(&StubOsHealthObserver::new(obs));
        assert_eq!(a.status, "healthy");
        assert!(a.findings.is_empty());
    }

    #[test]
    fn reproduces_2026_08_18_crash_class() {
        let obs = super::types::OsObservation {
            softlockup_panic: false, // the exact blindness that froze the box
            thp_enabled: "always".to_string(),
            thp_defrag: "defer+madvise".to_string(),
            available_mem_mib: 512,
            reclaim_pressure_pct: 80,
            dmesg_softlockup_count: 37,
            dmesg_hung_task_count: 0,
            dmesg_oom_count: 0,
        };
        let a = assess_os_health_default(&StubOsHealthObserver::new(obs));
        assert_eq!(a.status, "unhealthy");
        let kinds: Vec<&str> = a.findings.iter().map(|f| f.kind.as_str()).collect();
        assert!(kinds.contains(&"softlockup_no_panic"));
        assert!(kinds.contains(&"thp_enabled_risky"));
        assert!(kinds.contains(&"thp_defrag_risky"));
        assert!(kinds.contains(&"memory_pressure"));
    }

    #[test]
    fn dmesg_softlockup_alone_is_an_anomaly() {
        let obs = observe::OsObservation {
            softlockup_panic: true,
            thp_enabled: "madvise".to_string(),
            thp_defrag: "madvise".to_string(),
            available_mem_mib: 16384,
            reclaim_pressure_pct: 5,
            dmesg_softlockup_count: 4,
            dmesg_hung_task_count: 0,
            dmesg_oom_count: 0,
        };
        let a = assess_os_health_default(&StubOsHealthObserver::new(obs));
        assert!(a
            .findings
            .iter()
            .any(|f| f.kind == "kernel_softlockup_detected"));
    }
}
