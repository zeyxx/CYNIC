use serde::Serialize;

use crate::deps_surface_model::DepsReport;

#[derive(Debug, Serialize)]
pub(crate) struct DepsAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) status: &'static str,
    pub(crate) summary: DepsAuditSummary,
    pub(crate) findings: Vec<DepsAuditFinding>,
    pub(crate) rules: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DepsAuditSummary {
    pub(crate) total_python_cli_authorities: usize,
    pub(crate) normalizable_now: usize,
    pub(crate) blocked_missing_uv: usize,
    pub(crate) blocked_runtime_principal: usize,
    pub(crate) blocked_non_auto_apply: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct DepsAuditFinding {
    pub(crate) dep_id: String,
    pub(crate) status: String,
    pub(crate) priority: u32,
    pub(crate) class: &'static str,
    pub(crate) canonical_update_command: String,
    pub(crate) observed_payload_provenance: Option<String>,
    pub(crate) expected_payload_provenance: Option<String>,
    pub(crate) normalizable_now: bool,
    pub(crate) requires_uv: bool,
    pub(crate) requires_runtime_principal: bool,
    pub(crate) can_auto_apply: bool,
    pub(crate) note: String,
}

pub(crate) fn build_deps_audit(report: &DepsReport) -> DepsAuditReport {
    let mut findings = report
        .deps
        .iter()
        .filter_map(|dep| {
            let authority = dep.canonical_update_authority.as_ref()?;
            let (class, priority) = if authority.normalizable_now && authority.can_auto_apply {
                ("normalizable_now", 10)
            } else if authority.requires_uv && !authority.normalizable_now {
                ("blocked_missing_uv", 20)
            } else if authority.requires_runtime_principal && !authority.normalizable_now {
                ("blocked_runtime_principal", 30)
            } else if !authority.can_auto_apply {
                ("blocked_non_auto_apply", 40)
            } else {
                ("deferred", 50)
            };
            Some(DepsAuditFinding {
                dep_id: dep.id.to_string(),
                status: dep.status.clone(),
                priority,
                class,
                canonical_update_command: authority.canonical_update_command.clone(),
                observed_payload_provenance: authority.observed_payload_provenance.clone(),
                expected_payload_provenance: authority.expected_payload_provenance.clone(),
                normalizable_now: authority.normalizable_now,
                requires_uv: authority.requires_uv,
                requires_runtime_principal: authority.requires_runtime_principal,
                can_auto_apply: authority.can_auto_apply,
                note: authority.update_authority_note.clone(),
            })
        })
        .collect::<Vec<_>>();

    findings.sort_by_key(|finding| (finding.priority, finding.dep_id.clone()));

    let summary = DepsAuditSummary {
        total_python_cli_authorities: findings.len(),
        normalizable_now: findings
            .iter()
            .filter(|finding| finding.class == "normalizable_now")
            .count(),
        blocked_missing_uv: findings
            .iter()
            .filter(|finding| finding.class == "blocked_missing_uv")
            .count(),
        blocked_runtime_principal: findings
            .iter()
            .filter(|finding| finding.class == "blocked_runtime_principal")
            .count(),
        blocked_non_auto_apply: findings
            .iter()
            .filter(|finding| finding.class == "blocked_non_auto_apply")
            .count(),
    };

    let status = if summary.blocked_missing_uv > 0 || summary.blocked_runtime_principal > 0 {
        "attention_required"
    } else if summary.normalizable_now > 0 {
        "actionable"
    } else {
        "ok"
    };

    DepsAuditReport {
        schema: "cynic.ecosystem_deps_audit.v1",
        surface: "tech_watch",
        status,
        summary,
        findings,
        rules: vec![
            "audit classifies executable normalization state from canonical authority views",
            "payload provenance and update mutator remain distinct facts",
            "normalizable_now means the canonical command can be attempted under current local constraints",
        ],
    }
}
