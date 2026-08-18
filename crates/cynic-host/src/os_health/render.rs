//! CLI projection only. No policy, no I/O — render an `OsHealthAssessment`
//! produced by the decision layer into a compact human-readable report.

use super::types::OsHealthAssessment;

pub fn render_os_health_human(assessment: &OsHealthAssessment) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "OS substrate health: {}\n",
        assessment.status.to_uppercase()
    ));
    out.push_str(&format!(
        "  softlockup_panic={}  thp_enabled={}  thp_defrag={}\n",
        assessment.observed.softlockup_panic,
        assessment.observed.thp_enabled,
        assessment.observed.thp_defrag,
    ));
    out.push_str(&format!(
        "  available_mem_mib={}  reclaim_pressure_pct={}\n",
        assessment.observed.available_mem_mib, assessment.observed.reclaim_pressure_pct,
    ));
    out.push_str(&format!(
        "  dmesg: softlockup={} hung_task={} oom={}\n",
        assessment.observed.dmesg_softlockup_count,
        assessment.observed.dmesg_hung_task_count,
        assessment.observed.dmesg_oom_count,
    ));

    if assessment.findings.is_empty() {
        out.push_str("  findings: none\n");
    } else {
        out.push_str("  findings:\n");
        for f in &assessment.findings {
            out.push_str(&format!("    [{}] {}: {}\n", f.severity, f.kind, f.message));
            out.push_str(&format!("        fix: {}\n", f.remediation));
        }
    }

    if !assessment.unknowns.is_empty() {
        out.push_str(&format!("  unknowns: {}\n", assessment.unknowns.join(", ")));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os_health::decision::evaluate_os_health;
    use crate::os_health::default_os_health_config;
    use crate::os_health::observe::StubOsHealthObserver;
    use crate::os_health::types::OsObservation;

    #[test]
    fn render_includes_status_and_findings() {
        let obs = OsObservation {
            softlockup_panic: false,
            thp_enabled: "always".to_string(),
            thp_defrag: "defer+madvise".to_string(),
            available_mem_mib: 512,
            reclaim_pressure_pct: 80,
            dmesg_softlockup_count: 1,
            dmesg_hung_task_count: 0,
            dmesg_oom_count: 0,
        };
        let a = evaluate_os_health(&default_os_health_config(), &obs);
        let r = render_os_health_human(&a);
        assert!(r.contains("UNHEALTHY"));
        assert!(r.contains("softlockup_no_panic"));
        assert!(r.contains("fix:"));
    }

    // keep the stub import exercised
    #[allow(dead_code)]
    fn _use_stub() -> StubOsHealthObserver {
        StubOsHealthObserver::new(OsObservation {
            softlockup_panic: true,
            thp_enabled: "madvise".to_string(),
            thp_defrag: "madvise".to_string(),
            available_mem_mib: 8192,
            reclaim_pressure_pct: 5,
            dmesg_softlockup_count: 0,
            dmesg_hung_task_count: 0,
            dmesg_oom_count: 0,
        })
    }
}
