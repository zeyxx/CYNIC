pub mod path_rules;
pub mod topology;
pub mod rules;
pub mod scanner;

use std::path::Path;

use cynic_protocol::{AUDIT_REPORT_SCHEMA_V2, AuditReportV2, AuditSeverity, AuditSummary};

use self::path_rules::{DeclaredPathExistenceRule, PathSsotIgnoreRule, RustPathReferenceRule};
use self::topology::CrateTopologyRule;
use self::rules::{
    AuditRule, HotspotThresholdRule, KernelBoundaryRule, NodeHostBoundaryRule, OwnershipRule,
    PathSanitationRule,
};
use self::scanner::scan_workspace;

/// Recover the rule id that produced a violation code.
///
/// Codes are `<RULE_ID>-<DETAIL>` where the rule id is itself two dash-separated
/// segments (`SEC-001-ABSOLUTE-PATH` → `SEC-001`). Matching pass/fail on
/// `category` instead of rule id is wrong: several rules legitimately share a
/// category, so one critical finding would mark every sibling rule as failed and
/// silently deflate the maturity score.
fn rule_id_of(code: &str) -> &str {
    let mut segments = code.splitn(3, '-');
    let scope = segments.next().unwrap_or_default();
    let number = segments.next().unwrap_or_default();
    if scope.is_empty() || number.is_empty() {
        return code;
    }
    code.get(..scope.len() + 1 + number.len()).unwrap_or(code)
}

pub fn default_rules() -> Vec<Box<dyn AuditRule>> {
    vec![
        Box::new(PathSanitationRule),
        Box::new(HotspotThresholdRule),
        Box::new(KernelBoundaryRule),
        Box::new(NodeHostBoundaryRule),
        Box::new(OwnershipRule),
        Box::new(DeclaredPathExistenceRule),
        Box::new(RustPathReferenceRule),
        Box::new(PathSsotIgnoreRule),
        Box::new(CrateTopologyRule),
    ]
}

/// The path rules alone, for a focused `cynic audit paths` projection.
///
/// A surface that reports the whole workspace report would answer a question the
/// operator did not ask, and would make the path verdict unreadable inside
/// unrelated debt. The rules still run in `default_rules`, so this is a narrower
/// lens on the same evaluation, never a weaker one.
pub fn default_path_rules() -> Vec<Box<dyn AuditRule>> {
    vec![
        Box::new(DeclaredPathExistenceRule),
        Box::new(RustPathReferenceRule),
        Box::new(PathSsotIgnoreRule),
    ]
}

/// Evaluate an explicit rule set against an explicit workspace root.
fn run_rules(root: &Path, rules: Vec<Box<dyn AuditRule>>) -> AuditReportV2 {
    let index = scan_workspace(root);

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
                .any(|v| v.severity == AuditSeverity::Critical && rule_id_of(&v.code) == spec.id)
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

/// Full workspace audit: every default rule, including the path rules.
pub fn run_workspace_audit(root: &Path) -> AuditReportV2 {
    run_rules(root, default_rules())
}

/// Path reference audit only: declared paths, cited paths, and SSOT ignore state.
pub fn run_path_audit(root: &Path) -> AuditReportV2 {
    run_rules(root, default_path_rules())
}
