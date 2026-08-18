//! Pure decision layer for OS substrate health.
//!
//! Given the desired authority (`OsHealthConfig`) and the observed substrate
//! (`OsObservation`), produce findings with no I/O. The rule is: every known
//! risk becomes an explicit finding; every unreadable signal becomes an
//! explicit `unknown` rather than a silent pass.

use super::types::{OsHealthAssessment, OsHealthConfig, OsHealthFinding, OsObservation};
use crate::os_health::OS_HEALTH_SCHEMA;
use cynic_protocol::error_catalog::CynicErrorCode;
use cynic_protocol::health_signal::{OsHealthAnomaly, OS_HEALTH_ANOMALY_SCHEMA};

/// Map an internal finding kind to its sovereign CYNIC error code.
///
/// This is the bridge that turns a raw substrate observation into a stable,
/// operator-addressable code (mined from the 2026-08-18 soft-lockup incident).
pub fn finding_to_error_code(kind: &str) -> Option<CynicErrorCode> {
    match kind {
        "softlockup_no_panic" => Some(CynicErrorCode::OsSoftlockupPanicDisabled),
        "thp_enabled_risky" => Some(CynicErrorCode::OsTransparentHugepageDefragAggressive),
        "thp_defrag_risky" => Some(CynicErrorCode::OsTransparentHugepageDefragAggressive),
        "memory_pressure" => Some(CynicErrorCode::OsMemoryAvailableBelowFloor),
        "reclaim_pressure" => Some(CynicErrorCode::OsReclaimPressureAboveCeiling),
        "kernel_softlockup_detected" => Some(CynicErrorCode::OsKernelSoftlockupDetected),
        "kernel_hung_task" => Some(CynicErrorCode::OsKernelHungTaskDetected),
        "oom_events" => Some(CynicErrorCode::OsKernelOomDetected),
        _ => None,
    }
}

/// Collect the sovereign error codes present in an assessment, in finding order.
/// Downstream surfaces (kernel anomaly stream, CLI report) consume these.
pub fn assessment_error_codes(assessment: &OsHealthAssessment) -> Vec<CynicErrorCode> {
    assessment
        .findings
        .iter()
        .filter_map(|f| finding_to_error_code(&f.kind))
        .collect()
}

/// Convert the assessment's findings into portable kernel-ready anomalies
/// (`OsHealthAnomaly`, schema `cynic.os_health.anomaly.v1`). The kernel ingests
/// these via `POST /os-health/report` and re-emits them as `KernelEvent::Anomaly`
/// on its SSE/anomaly stream — closing the blindness that hid the 2026-08-18 hang.
pub fn assessment_to_anomalies(assessment: &OsHealthAssessment) -> Vec<OsHealthAnomaly> {
    assessment
        .findings
        .iter()
        .map(|f| OsHealthAnomaly {
            schema: OS_HEALTH_ANOMALY_SCHEMA.to_string(),
            error_code: f.error_code.clone(),
            kind: f.kind.clone(),
            severity: f.severity.clone(),
            message: f.message.clone(),
            remediation: f.remediation.clone(),
        })
        .collect()
}

pub fn evaluate_os_health(config: &OsHealthConfig, observed: &OsObservation) -> OsHealthAssessment {
    let mut findings: Vec<OsHealthFinding> = Vec::new();
    let mut unknowns: Vec<String> = Vec::new();

    // Helper to attach the sovereign code at the same place we build the finding.
    let mut push_finding = |kind: &str, severity: &str, message: String, remediation: String| {
        let code = finding_to_error_code(kind).map(|c| c.code().to_string());
        findings.push(OsHealthFinding {
            kind: kind.to_string(),
            severity: severity.to_string(),
            message,
            error_code: code,
            remediation,
        });
    };

    // 1. Kernel will not panic on soft lockup — the exact 2026-08-18 blindness.
    if config.softlockup_panic_required {
        if observed.softlockup_panic {
            // desired state satisfied
        } else {
            push_finding(
                "softlockup_no_panic",
                "critical",
                "kernel.softlockup_panic=0: a soft lockup freezes the machine instead of \
                 rebooting into a clean state (root cause class of the 2026-08-18 hang)"
                    .to_string(),
                "sysctl -w kernel.softlockup_panic=1 (and persist in /etc/sysctl.d) so a \
                 stuck kcompactd/CPU reboots instead of wedging the host"
                    .to_string(),
            );
        }
    }

    // 2. THP enabled policy
    if config.thp_enabled_forbid.contains(&observed.thp_enabled) {
        push_finding(
            "thp_enabled_risky",
            "critical",
            format!(
                "transparent_hugepage.enabled='{}' invites synchronous defrag stalls under \
                 memory pressure (kcompactd soft lockup class)",
                observed.thp_enabled
            ),
            "set /sys/kernel/mm/transparent_hugepage/enabled to madvise or never (echo \
             madvise > .../enabled)"
                .to_string(),
        );
    } else if observed.thp_enabled == "unknown" {
        unknowns.push("thp_enabled_unreadable".to_string());
    }

    // 3. THP defrag policy
    if config.thp_defrag_forbid.contains(&observed.thp_defrag) {
        push_finding(
            "thp_defrag_risky",
            "warning",
            format!(
                "transparent_hugepage.defrag='{}' performs direct reclaim/compaction on fault, \
                 stalling latency-sensitive tasks",
                observed.thp_defrag
            ),
            "set /sys/kernel/mm/transparent_hugepage/defrag to madvise or never (or \
             defer for background only)"
                .to_string(),
        );
    } else if observed.thp_defrag == "unknown" {
        unknowns.push("thp_defrag_unreadable".to_string());
    }

    // 4. Memory pressure (available floor)
    if observed.available_mem_mib < config.available_mem_mib_floor {
        push_finding(
            "memory_pressure",
            "warning",
            format!(
                "available memory {} MiB below floor {} MiB; reclaim pressure {}%",
                observed.available_mem_mib, config.available_mem_mib_floor,
                observed.reclaim_pressure_pct
            ),
            "reduce parallel heavy workloads (chrome + rustc builds + LLM server) or add \
             swap headroom; coordinate via runtime substrate, not ad-hoc kills"
                .to_string(),
        );
    } else if observed.reclaim_pressure_pct > config.reclaim_pressure_pct_ceiling {
        push_finding(
            "reclaim_pressure",
            "warning",
            format!(
                "reclaim pressure {}% above ceiling {}%",
                observed.reclaim_pressure_pct, config.reclaim_pressure_pct_ceiling
            ),
            "inspect memory consumers; avoid concurrent memory-heavy jobs".to_string(),
        );
    }

    // 5. Kernel log evidence of past/freeze events
    if observed.dmesg_softlockup_count > 0 {
        let severity = if observed.softlockup_panic {
            "warning"
        } else {
            "critical"
        };
        push_finding(
            "kernel_softlockup_detected",
            severity,
            format!(
                "kernel log shows {} soft-lockup event(s)",
                observed.dmesg_softlockup_count
            ),
            "correlate with THP/memory findings; treat as read-back proof of a substrate \
             fault, not a CYNIC bug"
                .to_string(),
        );
    }
    if observed.dmesg_hung_task_count > 0 {
        push_finding(
            "kernel_hung_task",
            "critical",
            format!(
                "kernel log shows {} hung-task/blocked-task event(s)",
                observed.dmesg_hung_task_count
            ),
            "investigate the blocking subsystem; likely memory reclaim or I/O stall".to_string(),
        );
    }
    if observed.dmesg_oom_count > 0 {
        push_finding(
            "oom_events",
            "warning",
            format!(
                "kernel log shows {} OOM-kill event(s)",
                observed.dmesg_oom_count
            ),
            "raise headroom or constrain workloads; OOM indicates the pressure ceiling was \
             crossed"
                .to_string(),
        );
    }

    let status = if findings.iter().any(|f| f.severity == "critical") {
        "unhealthy"
    } else if findings.iter().any(|f| f.severity == "warning") {
        "degraded"
    } else {
        "healthy"
    };

    OsHealthAssessment {
        schema: OS_HEALTH_SCHEMA.to_string(),
        status: status.to_string(),
        observed: observed.clone(),
        findings,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os_health::default_os_health_config;

    fn obs() -> OsObservation {
        OsObservation {
            softlockup_panic: false,
            thp_enabled: "always".to_string(),
            thp_defrag: "defer+madvise".to_string(),
            available_mem_mib: 512,
            reclaim_pressure_pct: 80,
            dmesg_softlockup_count: 37,
            dmesg_hung_task_count: 0,
            dmesg_oom_count: 0,
        }
    }

    #[test]
    fn critical_finding_drives_unhealthy() {
        let a = evaluate_os_health(&default_os_health_config(), &obs());
        assert_eq!(a.status, "unhealthy");
        assert!(a.findings.iter().any(|f| f.kind == "softlockup_no_panic"));
        assert!(a.findings.iter().any(|f| f.kind == "thp_enabled_risky"));
    }

    #[test]
    fn warning_only_drives_degraded() {
        let mut o = obs();
        o.softlockup_panic = true;
        o.thp_enabled = "madvise".to_string();
        o.dmesg_softlockup_count = 0;
        let a = evaluate_os_health(&default_os_health_config(), &o);
        assert_eq!(a.status, "degraded");
        assert!(a.findings.iter().all(|f| f.severity != "critical"));
    }

    #[test]
    fn unknown_thp_is_preserved_not_passed() {
        let mut o = obs();
        o.softlockup_panic = true;
        o.thp_enabled = "unknown".to_string();
        o.thp_defrag = "unknown".to_string();
        o.available_mem_mib = 8192;
        o.reclaim_pressure_pct = 10;
        o.dmesg_softlockup_count = 0;
        let a = evaluate_os_health(&default_os_health_config(), &o);
        assert!(a.unknowns.contains(&"thp_enabled_unreadable".to_string()));
        assert!(a.unknowns.contains(&"thp_defrag_unreadable".to_string()));
        assert_eq!(a.status, "healthy");
    }

    #[test]
    fn findings_carry_sovereign_error_codes() {
        let a = evaluate_os_health(&default_os_health_config(), &obs());
        // The 2026-08-18 root cause must map to CYNIC-OSH-001.
        let softlockup = a
            .findings
            .iter()
            .find(|f| f.kind == "softlockup_no_panic")
            .expect("softlockup finding present");
        assert_eq!(softlockup.error_code.as_deref(), Some("CYNIC-OSH-001"));
        // All mapped findings expose a sovereign code.
        for f in &a.findings {
            assert!(f.error_code.is_some(), "finding {} missing code", f.kind);
        }
        let codes = assessment_error_codes(&a);
        assert!(codes.contains(&cynic_protocol::error_catalog::CynicErrorCode::OsSoftlockupPanicDisabled));
    }
}
