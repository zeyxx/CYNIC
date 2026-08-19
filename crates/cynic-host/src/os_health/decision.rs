//! Pure decision layer for OS substrate health.
//!
//! Given the desired authority (`OsHealthConfig`) and the observed substrate
//! (`OsObservation`), produce findings with no I/O. The rule is: every known
//! risk becomes an explicit finding; every unreadable signal becomes an
//! explicit `unknown` rather than a silent pass.

use super::types::{OsHealthAssessment, OsHealthConfig, OsHealthFinding, OsObservation};
use crate::os_health::OS_HEALTH_SCHEMA;

pub fn evaluate_os_health(config: &OsHealthConfig, observed: &OsObservation) -> OsHealthAssessment {
    let mut findings: Vec<OsHealthFinding> = Vec::new();
    let mut unknowns: Vec<String> = Vec::new();

    // 1. Kernel will not panic on soft lockup — the exact 2026-08-18 blindness.
    if config.softlockup_panic_required {
        if observed.softlockup_panic {
            // desired state satisfied
        } else {
            findings.push(OsHealthFinding {
                kind: "softlockup_no_panic".to_string(),
                severity: "critical".to_string(),
                message:
                    "kernel.softlockup_panic=0: a soft lockup freezes the machine instead of \
                     rebooting into a clean state (root cause class of the 2026-08-18 hang)"
                        .to_string(),
                remediation:
                    "sysctl -w kernel.softlockup_panic=1 (and persist in /etc/sysctl.d) so a \
                     stuck kcompactd/CPU reboots instead of wedging the host"
                        .to_string(),
            });
        }
    }

    // 2. THP enabled policy
    if config.thp_enabled_forbid.contains(&observed.thp_enabled) {
        findings.push(OsHealthFinding {
            kind: "thp_enabled_risky".to_string(),
            severity: "critical".to_string(),
            message: format!(
                "transparent_hugepage.enabled='{}' invites synchronous defrag stalls under \
                 memory pressure (kcompactd soft lockup class)",
                observed.thp_enabled
            ),
            remediation:
                "set /sys/kernel/mm/transparent_hugepage/enabled to madvise or never (echo \
                 madvise > .../enabled)"
                    .to_string(),
        });
    } else if observed.thp_enabled == "unknown" {
        unknowns.push("thp_enabled_unreadable".to_string());
    }

    // 3. THP defrag policy
    if config.thp_defrag_forbid.contains(&observed.thp_defrag) {
        findings.push(OsHealthFinding {
            kind: "thp_defrag_risky".to_string(),
            severity: "warning".to_string(),
            message: format!(
                "transparent_hugepage.defrag='{}' performs direct reclaim/compaction on fault, \
                 stalling latency-sensitive tasks",
                observed.thp_defrag
            ),
            remediation:
                "set /sys/kernel/mm/transparent_hugepage/defrag to madvise or never (or \
                 defer for background only)"
                    .to_string(),
        });
    } else if observed.thp_defrag == "unknown" {
        unknowns.push("thp_defrag_unreadable".to_string());
    }

    // 4. Memory pressure (available floor)
    if observed.available_mem_mib < config.available_mem_mib_floor {
        findings.push(OsHealthFinding {
            kind: "memory_pressure".to_string(),
            severity: "warning".to_string(),
            message: format!(
                "available memory {} MiB below floor {} MiB; reclaim pressure {}%",
                observed.available_mem_mib, config.available_mem_mib_floor,
                observed.reclaim_pressure_pct
            ),
            remediation:
                "reduce parallel heavy workloads (chrome + rustc builds + LLM server) or add \
                 swap headroom; coordinate via runtime substrate, not ad-hoc kills"
                    .to_string(),
        });
    } else if observed.reclaim_pressure_pct > config.reclaim_pressure_pct_ceiling {
        findings.push(OsHealthFinding {
            kind: "reclaim_pressure".to_string(),
            severity: "warning".to_string(),
            message: format!(
                "reclaim pressure {}% above ceiling {}%",
                observed.reclaim_pressure_pct, config.reclaim_pressure_pct_ceiling
            ),
            remediation: "inspect memory consumers; avoid concurrent memory-heavy jobs"
                .to_string(),
        });
    }

    // 5. Kernel log evidence of past/freeze events
    if observed.dmesg_softlockup_count > 0 {
        findings.push(OsHealthFinding {
            kind: "kernel_softlockup_detected".to_string(),
            severity: if observed.softlockup_panic {
                "warning"
            } else {
                "critical"
            }
            .to_string(),
            message: format!(
                "kernel log shows {} soft-lockup event(s)",
                observed.dmesg_softlockup_count
            ),
            remediation:
                "correlate with THP/memory findings; treat as read-back proof of a substrate \
                 fault, not a CYNIC bug"
                    .to_string(),
        });
    }
    if observed.dmesg_hung_task_count > 0 {
        findings.push(OsHealthFinding {
            kind: "kernel_hung_task".to_string(),
            severity: "critical".to_string(),
            message: format!(
                "kernel log shows {} hung-task/blocked-task event(s)",
                observed.dmesg_hung_task_count
            ),
            remediation: "investigate the blocking subsystem; likely memory reclaim or I/O stall"
                .to_string(),
        });
    }
    if observed.dmesg_oom_count > 0 {
        findings.push(OsHealthFinding {
            kind: "oom_events".to_string(),
            severity: "warning".to_string(),
            message: format!(
                "kernel log shows {} OOM-kill event(s)",
                observed.dmesg_oom_count
            ),
            remediation:
                "raise headroom or constrain workloads; OOM indicates the pressure ceiling was \
                 crossed"
                    .to_string(),
        });
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
}
