mod artefact_rule;
mod debt_pattern_rule;
mod module_inversion_rule;
mod runtime_divergence;
pub mod branch_audit;
pub mod debt_ledger;
pub mod growth;
pub mod history;
pub mod node_host_audit;
pub mod path_rules;
pub mod ports;
pub mod refactor_risk;
pub use refactor_risk::{assess, RefactorCandidate, RefactorRisk, RefactorRiskLevel};
pub mod prose_citation_rule;
pub mod retired_metaphor_rule;
pub mod rules;
pub mod runtime_path_literal_rule;
pub mod scanner;
pub mod topology;
pub mod topology_generate;

use std::path::Path;
use cynic_protocol::{AUDIT_REPORT_SCHEMA_V2, AuditReportV2, AuditSeverity, AuditSummary};

use self::artefact_rule::{ArtefactContractRule, AutobinsFrozenRule};
use self::debt_pattern_rule::{DebtPatternRule, GodCrateBudgetRule, UnnormedVocabularyDriftRule};
use self::module_inversion_rule::{LayerModelDeclaredRule, ModuleInversionRule};
use self::path_rules::{DeclaredPathExistenceRule, PathSsotIgnoreRule, RustPathReferenceRule};
use self::prose_citation_rule::CodeCitesProseRule;
use self::retired_metaphor_rule::RetiredMetaphorRule;
use self::topology::CrateTopologyRule;
use self::rules::{
    AuditRule, HostProbeLeakRule, HotspotThresholdRule, KernelBoundaryRule, NodeHostBoundaryRule,
    OwnershipRule, PathSanitationRule,
};
use self::runtime_divergence::RuntimeDivergenceRule;
use self::runtime_path_literal_rule::RuntimePathLiteralRule;
use self::scanner::scan_workspace;
pub use self::scanner::{HostObservation, WorkspaceSourceIndex};
pub use self::debt_pattern_rule::{Arch005Rule, Arch006Rule, GodCrateBudgetExceededRule, VocabularyDriftRule};
pub use debt_ledger::reconcile_debt_ledger;

/// The RUN-001 body, callable from a fixture.
///
/// Exposed so the runtime-divergence checks can be proven against a constructed
/// observation instead of against whichever host happens to run the test. A
/// security check that can only be exercised by breaking the real machine is one
/// nobody exercises.
pub fn evaluate_runtime_divergence(index: &WorkspaceSourceIndex) -> Vec<cynic_protocol::AuditViolation> {
    use self::runtime_divergence::AuditRule as _;
    self::runtime_divergence::RuntimeDivergenceRule.evaluate(index)
}

/// Recover the rule id that produced a violation code.
///
/// Codes are `<RULE_ID>-<DETAIL>` (`SEC-001-ABSOLUTE-PATH` → `SEC-001`).
/// Matching pass/fail on `category` instead of rule id is wrong.
fn rule_id_of(code: &str) -> &str {
    let mut s = code.splitn(3, '-');
    match (s.next(), s.next()) {
        (Some(sc), Some(num)) if !sc.is_empty() && !num.is_empty() => {
            code.get(..sc.len() + 1 + num.len()).unwrap_or(code)
        }
        _ => code,
    }
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
        Box::new(RuntimeDivergenceRule),
        Box::new(ArtefactContractRule),
        Box::new(AutobinsFrozenRule),
        Box::new(ModuleInversionRule),
        Box::new(LayerModelDeclaredRule),
        Box::new(CodeCitesProseRule),
        Box::new(RuntimePathLiteralRule),
        Box::new(RetiredMetaphorRule),
        Box::new(HostProbeLeakRule),
        Box::new(DebtPatternRule),
        Box::new(GodCrateBudgetRule),
        Box::new(UnnormedVocabularyDriftRule),
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
        Box::new(RuntimePathLiteralRule),
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

/// Full workspace audit: every default rule, including the path rules, with the
/// declared debt ledger applied.
///
/// The ledger is applied HERE, at the single entry point every consumer already
/// calls, rather than at each call site. A comparator wired per call site is a
/// comparator that one call site forgets.
///
/// After the ledger pass the report holds only what requires action now:
/// undeclared debt, debt that grew, stale ledger entries, and an unreadable
/// ledger. The declared violations are removed because their record is the
/// ledger itself. See the `debt_ledger` module for why a threshold is not the
/// alternative.
/// The default rules over the workspace, with NO debt ledger applied.
///
/// The counterpart of [`run_workspace_audit`], not a replacement for it. That
/// function answers "what must an operator act on", and it answers it by
/// absorbing every violation the ledger already declares — which is the correct
/// behaviour for a verdict and useless for a measurement.
///
/// A shrink-only gate needs the facts before that absorption: `layer_debt` in
/// `infra/crate-topology.json` records fifteen ascending edges, all declared in
/// the debt ledger, so the verdict contains none of them. A ratchet that reads
/// the verdict compares fifteen against zero and fails for a reason that has
/// nothing to do with the architecture.
///
/// Both run the same rules over the same index; this one stops before the
/// ledger rewrites the report.
pub fn evaluate_workspace_rules(root: &Path) -> AuditReportV2 {
    run_rules(root, default_rules())
}

pub fn run_workspace_audit(root: &Path) -> AuditReportV2 {
    let mut report = run_rules(root, default_rules());
    debt_ledger::apply_debt_ledger(&mut report, root);
    report
}

/// Path reference audit only: declared paths, cited paths, and SSOT ignore state.
pub fn run_path_audit(root: &Path) -> AuditReportV2 {
    run_rules(root, default_path_rules())
}

pub use history::{append_audit_history, read_audit_history};
