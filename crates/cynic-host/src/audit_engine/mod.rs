pub mod rules;
pub mod scanner;

use std::path::Path;

use cynic_protocol::{
    AUDIT_REPORT_SCHEMA_V2, AuditReportV2, AuditSeverity, AuditSummary,
};

use self::rules::{
    AuditRule, HotspotThresholdRule, KernelBoundaryRule, LayerBoundaryRule, NodeHostBoundaryRule,
    OwnershipRule, PathSanitationRule,
};
use self::scanner::scan_workspace;

pub fn default_rules() -> Vec<Box<dyn AuditRule>> {
    vec![
        Box::new(PathSanitationRule),
        Box::new(HotspotThresholdRule),
        Box::new(LayerBoundaryRule),
        Box::new(KernelBoundaryRule),
        Box::new(NodeHostBoundaryRule),
        Box::new(OwnershipRule),
    ]
}

pub fn run_workspace_audit(root: &Path) -> AuditReportV2 {
    let index = scan_workspace(root);
    let rules = default_rules();

    let mut evaluated_specs = Vec::new();
    let mut all_violations = Vec::new();

    for rule in &rules {
        evaluated_specs.push(rule.spec());
        let mut violations = rule.evaluate(&index);
        all_violations.append(&mut violations);
    }

    let critical_count = all_violations
        .iter()
        .filter(|v| v.severity == AuditSeverity::Critical)
        .count();
    let warning_count = all_violations
        .iter()
        .filter(|v| v.severity == AuditSeverity::Warning)
        .count();
    let debt_count = all_violations
        .iter()
        .filter(|v| v.severity == AuditSeverity::Debt)
        .count();

    let passed_rules_count = rules
        .iter()
        .filter(|r| {
            let spec = r.spec();
            !all_violations
                .iter()
                .any(|v| v.severity == AuditSeverity::Critical && v.category == spec.category)
        })
        .count();

    let status = if critical_count == 0 {
        if debt_count > 0 || warning_count > 0 {
            "has_debt".to_string()
        } else {
            "ok".to_string()
        }
    } else {
        "blocked".to_string()
    };

    let maturity_score = if rules.is_empty() {
        100
    } else {
        ((passed_rules_count as f64 / rules.len() as f64) * 100.0) as u8
    };

    AuditReportV2 {
        schema: AUDIT_REPORT_SCHEMA_V2.to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        summary: AuditSummary {
            status,
            total_rules_evaluated: rules.len(),
            passed_rules_count,
            critical_count,
            warning_count,
            debt_count,
            residual_unknown_count: 0,
            maturity_score_percent: maturity_score,
        },
        violations: all_violations,
        evaluated_rules: evaluated_specs,
        residual_unknowns: Vec::new(),
    }
}
