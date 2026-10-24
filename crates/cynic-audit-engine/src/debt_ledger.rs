//! Audit debt ledger: the declared, shrink-only record of known violations.
//!
//! WHY. `run_workspace_audit` reports 224 violations (23 critical) on the
//! current tree. A gate that is red on arrival blocks all work; a gate that
//! ignores them is a phantom gate. The ledger is the third option: the debt is
//! declared and counted, any NEW violation fails, and any PAID debt must be
//! removed. It can only shrink.
//!
//! WHY NOT A THRESHOLD. A threshold ("tolerate up to 250") is a policy that
//! erodes: it tolerates the next violation by construction. An exact
//! per-identity count does not, because any increase is a distinct, named
//! failure.
//!
//! WHY A POST-PASS AND NOT A RULE. The ledger's input is the complete set of
//! violations produced by the other rules. A rule receives only the
//! `WorkspaceSourceIndex` and cannot see its siblings' findings, so a ledger
//! rule would be structurally unable to do its job. This runs after
//! `run_workspace_audit`, which is the only place the full set exists.
//!
//! NEGATIVE BOUNDARY. It does not fix anything and does not decide policy. It
//! compares the observed set with the declared set and names the difference in
//! both directions.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use cynic_protocol::{AuditCategory, AuditReportV2, AuditSeverity, AuditSpan, AuditViolation};

pub const LEDGER_PATH: &str = "infra/audit-debt-ledger.json";

/// Rules whose message embeds a measurement rather than a name.
///
/// `ARCH-001-HOTSPOT` says "File X has 435 lines", and 435 changes on any edit.
/// Keying on that message would force ledger regeneration on every edit to any
/// of 199 files, turning a deliberate act into muscle memory and eroding the
/// signal the gate exists to carry. For these, the file is the identity.
///
/// Every other rule's message names a path, a unit, or a service -- stable, and
/// necessary: the 15 `ARCH-004-UPWARD-EDGE` violations all share
/// `span.file_path = infra/crate-topology.json` and differ only by message, so
/// keying on the file would collapse 15 distinct contract violations into one
/// and hide a partial repair.
const MEASURED_RULES: &[&str] = &["ARCH-001-HOTSPOT"];

/// The stable identity of a violation, matching the ledger generator exactly.
pub fn identity(violation: &AuditViolation) -> String {
    if MEASURED_RULES.contains(&violation.code.as_str()) {
        let file = violation
            .span
            .as_ref()
            .map(|s| s.file_path.as_str())
            .unwrap_or("");
        return format!("{}|{}", violation.code, file);
    }
    format!("{}|{}", violation.code, violation.message)
}

/// What the ledger comparison found.
#[derive(Debug, Clone)]
pub struct LedgerVerdict {
    /// Violations matched to a ledger entry at or below its recorded count.
    pub accounted: usize,
    /// Identities observed but absent from the ledger, or above their count.
    /// Each is a new or regressed violation: the gate must fail.
    pub unaccounted: Vec<String>,
    /// Ledger entries with no observed violation. Debt paid but not recorded as
    /// paid: the gate must fail, because a stale ledger hides that it is stale.
    pub stale: Vec<String>,
}

impl LedgerVerdict {
    pub fn is_clean(&self) -> bool {
        self.unaccounted.is_empty() && self.stale.is_empty()
    }
}

/// Read the declared counts from the ledger.
///
/// `None` means the ledger could not be read or parsed -- an explicit unknown,
/// never a silent pass. A gate whose ledger is unreadable cannot compare, and
/// saying so is the correct outcome.
fn declared_counts(root: &Path) -> Option<BTreeMap<String, usize>> {
    let path = root.join(LEDGER_PATH);
    let text = fs::read_to_string(&path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entries = doc.get("entries")?.as_array()?;

    let mut counts = BTreeMap::new();
    for entry in entries {
        let ident = entry.get("identity")?.as_str()?.to_string();
        let count = entry.get("count")?.as_u64()? as usize;
        counts.insert(ident, count);
    }
    Some(counts)
}

/// Compare a report against the ledger, and replace the accounted debt with the
/// ledger's own findings.
///
/// After this call, `report.violations` holds only findings that require action
/// now: unaccounted debt, stale ledger entries, and any rule that reported
/// nothing (a clean rule contributes nothing). The 224 declared violations are
/// removed because they are declared -- their record is the ledger itself, and
/// repeating them in every report is what made the report unreadable.
pub fn apply_debt_ledger(report: &mut AuditReportV2, root: &Path) -> LedgerVerdict {
    let Some(declared) = declared_counts(root) else {
        let violation = AuditViolation {
            code: "LEDGER-003-UNREADABLE".to_string(),
            message: format!(
                "{LEDGER_PATH} could not be read or parsed. The debt ledger comparator \
                 could not run: this is an explicit unknown, not a pass."
            ),
            severity: AuditSeverity::Critical,
            category: AuditCategory::ResidualUnknown,
            span: None,
            remediation: Some(format!(
                "Restore {LEDGER_PATH} as valid JSON with an `entries` array of \
                 {{identity, count}}."
            )),
        };
        let unaccounted = vec![violation.code.clone()];
        report.violations = vec![violation];
        recompute_summary(report);
        return LedgerVerdict {
            accounted: 0,
            unaccounted,
            stale: Vec::new(),
        };
    };

    let mut observed: BTreeMap<String, usize> = BTreeMap::new();
    for violation in &report.violations {
        *observed.entry(identity(violation)).or_insert(0) += 1;
    }

    let mut unaccounted = Vec::new();
    let mut findings = Vec::new();

    for (ident, count) in &observed {
        match declared.get(ident) {
            None => {
                unaccounted.push(ident.clone());
                findings.push(AuditViolation {
                    code: "LEDGER-001-UNDECLARED-DEBT".to_string(),
                    message: format!(
                        "A violation with identity `{ident}` (observed {count}x) is absent \
                         from {LEDGER_PATH}. New debt must not be absorbed silently."
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::MutationContract,
                    span: None,
                    remediation: Some(format!(
                        "Fix the violation, or record it deliberately in {LEDGER_PATH} \
                         with its identity and count."
                    )),
                });
            }
            Some(&allowed) if count > &allowed => {
                unaccounted.push(ident.clone());
                findings.push(AuditViolation {
                    code: "LEDGER-002-DEBT-GREW".to_string(),
                    message: format!(
                        "Identity `{ident}` was recorded {allowed}x in {LEDGER_PATH} and is \
                         now observed {count}x. The ledger may only shrink."
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::MutationContract,
                    span: None,
                    remediation: Some(format!(
                        "Remove the {count_minus} new occurrence(s), or raise the count in \
                         {LEDGER_PATH} as a deliberate, reviewed act.",
                        count_minus = count - allowed
                    )),
                });
            }
            Some(_) => {}
        }
    }

    let mut stale = Vec::new();
    for ident in declared.keys() {
        if !observed.contains_key(ident) {
            stale.push(ident.clone());
            findings.push(AuditViolation {
                code: "LEDGER-004-STALE-ENTRY".to_string(),
                message: format!(
                    "{LEDGER_PATH} records `{ident}` but no rule observed it. Paid debt left \
                     in the ledger hides that the ledger is stale, and the next reader \
                     believes a fixed violation is still present."
                ),
                // Critical, not Warning: this repository's ratified shrink-only pattern
                // (orphan_sources_ratchet) fails in BOTH directions, and the reason is
                // the same here. A ledger that tolerates stale entries stops being a
                // record of what exists, which is the only thing that makes it worth
                // reading. The cost is that paying a debt requires editing the ledger in
                // the same change -- which is the discipline, not a side effect.
                severity: AuditSeverity::Critical,
                category: AuditCategory::MutationContract,
                span: Some(AuditSpan {
                    file_path: LEDGER_PATH.to_string(),
                    line_start: 1,
                    line_end: 1,
                    snippet: Some(ident.clone()),
                }),
                remediation: Some(format!(
                    "Delete the `{ident}` entry from {LEDGER_PATH}. The ledger may only shrink."
                )),
            });
        }
    }

    let accounted = observed
        .iter()
        .filter(|(ident, count)| {
            declared
                .get(*ident)
                .is_some_and(|allowed| *count <= allowed)
        })
        .map(|(_, count)| *count)
        .sum();

    report.violations = findings;
    recompute_summary(report);

    LedgerVerdict {
        accounted,
        unaccounted,
        stale,
    }
}

/// Recompute the summary from the violations now present.
///
/// Deliberately duplicated from `run_rules` rather than shared: that one counts
/// violations produced by rules, this one counts what survived the ledger. If
/// they diverged the report would describe a set it does not hold.
fn recompute_summary(report: &mut AuditReportV2) {
    let critical = report
        .violations
        .iter()
        .filter(|v| v.severity == AuditSeverity::Critical)
        .count();
    let warning = report
        .violations
        .iter()
        .filter(|v| v.severity == AuditSeverity::Warning)
        .count();
    let debt = report
        .violations
        .iter()
        .filter(|v| v.severity == AuditSeverity::Debt)
        .count();

    let passed = report
        .evaluated_rules
        .iter()
        .filter(|spec| {
            !report
                .violations
                .iter()
                .any(|v| v.severity == AuditSeverity::Critical && v.code.starts_with(&spec.id))
        })
        .count();

    let status = if critical > 0 {
        "blocked"
    } else if debt > 0 || warning > 0 {
        "has_debt"
    } else {
        "ok"
    };

    let total = report.evaluated_rules.len();
    report.summary.status = status.to_string();
    report.summary.critical_count = critical;
    report.summary.warning_count = warning;
    report.summary.debt_count = debt;
    report.summary.passed_rules_count = passed;
    report.summary.maturity_score_percent = if total == 0 {
        100
    } else {
        ((passed as f64 / total as f64) * 100.0) as u8
    };
}

/// Reconcile `infra/audit-debt-ledger.json` against the current physical substrate.
///
/// Removes paid/stale entries and records current observed violation identities.
pub fn reconcile_debt_ledger(root: &Path) -> Result<usize, String> {
    let raw_report = crate::evaluate_workspace_rules(root);
    let path = root.join(LEDGER_PATH);
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", LEDGER_PATH, e))?;
    let mut doc: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse {}: {}", LEDGER_PATH, e))?;

    let mut observed_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut observed_meta: BTreeMap<String, (String, &'static str)> = BTreeMap::new();

    for v in &raw_report.violations {
        let ident = identity(v);
        *observed_counts.entry(ident.clone()).or_insert(0) += 1;
        let parts: Vec<&str> = v.code.split('-').collect();
        let rule_id = if parts.len() >= 2 {
            format!("{}-{}", parts[0], parts[1])
        } else {
            v.code.clone()
        };
        let sev_str = match v.severity {
            AuditSeverity::Critical => "critical",
            AuditSeverity::Warning => "warning",
            AuditSeverity::Debt => "debt",
            AuditSeverity::Info => "info",
        };
        observed_meta.entry(ident).or_insert((rule_id, sev_str));
    }

    let mut new_entries = Vec::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_crit = 0;
    let mut total_warn = 0;
    let mut total_debt = 0;

    for (ident, count) in &observed_counts {
        let (rule_id, sev_str) = observed_meta
            .get(ident)
            .map(|(r, s)| (r.clone(), *s))
            .unwrap_or_else(|| ("ARCH-001".to_string(), "debt"));
        *by_rule.entry(rule_id.clone()).or_insert(0) += *count;
        match sev_str {
            "critical" => total_crit += *count,
            "warning" => total_warn += *count,
            _ => total_debt += *count,
        }

        let mut entry = serde_json::Map::new();
        entry.insert("identity".to_string(), serde_json::Value::String(ident.clone()));
        entry.insert("count".to_string(), serde_json::Value::Number((*count as u64).into()));
        entry.insert("rule".to_string(), serde_json::Value::String(rule_id));
        entry.insert("severity".to_string(), serde_json::Value::String(sev_str.to_string()));
        new_entries.push(serde_json::Value::Object(entry));
    }

    let measured_total = observed_counts.values().sum::<usize>();
    let measured_identities = observed_counts.len();

    let mut measured_obj = serde_json::Map::new();
    measured_obj.insert("note".to_string(), serde_json::Value::String("Reconciled with physical substrate".to_string()));
    measured_obj.insert("total".to_string(), serde_json::Value::Number((measured_total as u64).into()));
    measured_obj.insert("critical".to_string(), serde_json::Value::Number((total_crit as u64).into()));
    measured_obj.insert("debt".to_string(), serde_json::Value::Number((total_debt as u64).into()));
    measured_obj.insert("warning".to_string(), serde_json::Value::Number((total_warn as u64).into()));
    measured_obj.insert("identities".to_string(), serde_json::Value::Number((measured_identities as u64).into()));

    let mut by_rule_map = serde_json::Map::new();
    for (k, v) in by_rule {
        by_rule_map.insert(k, serde_json::Value::Number((v as u64).into()));
    }
    measured_obj.insert("by_rule".to_string(), serde_json::Value::Object(by_rule_map));

    if let Some(doc_obj) = doc.as_object_mut() {
        doc_obj.insert("measured".to_string(), serde_json::Value::Object(measured_obj));
        doc_obj.insert("entries".to_string(), serde_json::Value::Array(new_entries));
    }

    let formatted = serde_json::to_string_pretty(&doc)
        .map_err(|e| format!("failed to serialize updated ledger: {}", e))? + "\n";
    fs::write(&path, formatted)
        .map_err(|e| format!("failed to write {}: {}", LEDGER_PATH, e))?;

    Ok(measured_identities)
}
