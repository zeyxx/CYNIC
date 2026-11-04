//! Debt pattern rule: detects recurring architectural debt patterns
//! observed during vertical extractions (2026-10 to 2026-11).
//!
//! WHY. Each vertical extraction revealed the same classes of debt:
//!   - SSH direct (Command::new("ssh")) outside cynic-ssh
//!   - I/O in domain/ (std::fs, Command, TcpStream in pure modules)
//!   - Re-export shims (pub use crate::internal::* in lib.rs)
//!   - Upward deps (cynic-host depending on cynic-build/vcs/etc.)
//!   - Ghost files (adapters not referenced by any consumer)
//!
//! These patterns are now codified as declarative checks so the audit engine
//! serves as a compass for LLMs and operators before starting a new extraction.

use std::collections::BTreeSet;

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditViolation};

use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

const RULE_ID: &str = "DEBT-001";

pub struct DebtPatternRule;

impl AuditRule for DebtPatternRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: RULE_ID.to_string(),
            name: "debt_pattern".to_string(),
            description: "Detects recurring architectural debt patterns: SSH direct, I/O in domain/, re-export shims, upward deps, ghost files.".to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Warning,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        for file in &index.rust_files {
            let path = &file.relative_path;

            // Skip test files
            if path.contains("/tests/") || path.contains("_test.rs") {
                continue;
            }

            // Rule 1: SSH direct outside cynic-ssh
            if !path.contains("cynic-ssh") && file.content.contains("Command::new(\"ssh\")") {
                violations.push(AuditViolation {
                    code: "CYNIC-ARCH-003".to_string(),
                    message: format!(
                        "SSH direct (Command::new(\"ssh\")) found in `{}`. Must use cynic_ssh::argv::ssh_argv.",
                        path
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::Boundary,
                    span: None,
                    remediation: Some("Replace with cynic_ssh::argv::ssh_argv or ssh_argv_with_options.".to_string()),
                });
            }

            // Rule 2: I/O in domain/ (pure module must not do I/O)
            if path.contains("/domain/") {
                let has_io = file.content.contains("std::fs::")
                    || file.content.contains("std::process::Command")
                    || file.content.contains("TcpStream::")
                    || file.content.contains("std::net::");
                if has_io {
                    violations.push(AuditViolation {
                        code: "CYNIC-ARCH-002".to_string(),
                        message: format!(
                            "Forbidden I/O in domain/ module `{}`. Domain modules must be pure FCIS.",
                            path
                        ),
                        severity: AuditSeverity::Critical,
                        category: AuditCategory::Boundary,
                        span: None,
                        remediation: Some("Extract I/O to an adapter in adapters/ behind a port trait.".to_string()),
                    });
                }
            }

            // Rule 3: Re-export shims in lib.rs
            if path.ends_with("/lib.rs") {
                for line in file.content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("pub use crate::") && trimmed.contains("::") {
                        // Count depth: pub use crate::a::b::c is a shim
                        let parts: Vec<&str> = trimmed.split("::").collect();
                        if parts.len() >= 4 {
                            violations.push(AuditViolation {
                                code: "CYNIC-ARCH-001".to_string(),
                                message: format!(
                                    "Re-export shim in `{}`: `{}`. Canonicalize to direct module path.",
                                    path, trimmed
                                ),
                                severity: AuditSeverity::Warning,
                                category: AuditCategory::Boundary,
                                span: None,
                                remediation: Some("Replace with direct module path import.".to_string()),
                            });
                            break; // one per file
                        }
                    }
                }
            }

            // Rule 4: Upward deps (cynic-host depending on higher-layer crates)
            if path.contains("cynic-host") && path.ends_with("Cargo.toml") {
                let higher_crates = ["cynic-build", "cynic-vcs", "cynic-network", "cynic-surreal", "cynic-secrets", "cynic-eval"];
                for crate_name in &higher_crates {
                    if file.content.contains(&format!("{} = {{ path", crate_name)) {
                        violations.push(AuditViolation {
                            code: "CYNIC-ARCH-004".to_string(),
                            message: format!(
                                "Upward dependency: cynic-host depends on {} (higher layer).",
                                crate_name
                            ),
                            severity: AuditSeverity::Critical,
                            category: AuditCategory::Boundary,
                            span: None,
                            remediation: Some(format!("Remove dependency; invert via port trait.")),
                        });
                    }
                }
            }
        }

        // Rule 5: Ghost files (adapters not referenced)
        // Collect all adapter file names
        let mut adapter_files: BTreeSet<String> = BTreeSet::new();
        for file in &index.rust_files {
            if file.relative_path.contains("/adapters/") && file.relative_path.ends_with(".rs") {
                let name = file.relative_path.split('/').last().unwrap_or("").to_string();
                if name != "mod.rs" {
                    adapter_files.insert(name);
                }
            }
        }
        // Check if each adapter is referenced somewhere
        let all_content: String = index.rust_files.iter().map(|f| f.content.clone()).collect::<Vec<_>>().join("\n");
        for adapter in &adapter_files {
            let stem = adapter.trim_end_matches(".rs");
            // Check if the adapter module is declared in a mod.rs or referenced
            let mod_decl = format!("mod {};", stem);
            let use_decl = format!("{}::", stem);
            if !all_content.contains(&mod_decl) && !all_content.contains(&use_decl) {
                // Only report if it's in cynic-host (where ghost files accumulate)
                // This is a heuristic; may have false positives
                violations.push(AuditViolation {
                    code: format!("{RULE_ID}-GHOST-FILE"),
                    message: format!(
                        "Potential ghost file: `{}` not referenced by any mod/use declaration.",
                        adapter
                    ),
                    severity: AuditSeverity::Warning,
                    category: AuditCategory::Boundary,
                    span: None,
                    remediation: Some("Verify the file is still used; delete if orphaned.".to_string()),
                });
            }
        }

        violations
    }
}


// ---------------------------------------------------------------------------
// CYNIC-ARCH-005 — GodCrateBudgetExceeded
// ---------------------------------------------------------------------------

use serde_json::Value;

pub struct GodCrateBudgetRule;
pub struct GodCrateBudgetExceededRule;
pub type Arch005Rule = GodCrateBudgetRule;

impl AuditRule for GodCrateBudgetRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "CYNIC-ARCH-005".to_string(),
            name: "god_crate_budget_exceeded".to_string(),
            description: "A single crate must not exceed 45,000 LOC; extract domain verticals into DDD crates.".to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Warning,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        const BUDGET: u64 = 45_000;
        let mut violations = Vec::new();

        let mut found_topology = false;
        for file in &index.json_files {
            if !(file.relative_path == "infra/crate-topology.json"
                || file.relative_path.ends_with("crate-topology.json"))
            {
                continue;
            }
            found_topology = true;
            if let Ok(doc) = serde_json::from_str::<Value>(&file.content) {
                if let Some(crates) = doc.get("crates").and_then(|v| v.as_object()) {
                    for (name, body) in crates {
                        let loc = body.get("loc").and_then(|v| v.as_u64()).unwrap_or(0);
                        if loc > BUDGET {
                            violations.push(AuditViolation {
                                code: "CYNIC-ARCH-005".to_string(),
                                message: format!(
                                    "God crate budget exceeded: `{}` has {} LOC (limit {}).",
                                    name, loc, BUDGET
                                ),
                                severity: AuditSeverity::Warning,
                                category: AuditCategory::Boundary,
                                span: None,
                                remediation: Some(
                                    "Extract domain verticals into dedicated DDD crates to bring the crate under the 45,000 LOC budget.".to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }

        if !found_topology {
            use std::collections::BTreeMap;
            let mut locs: BTreeMap<String, usize> = BTreeMap::new();
            for file in &index.rust_files {
                if let Some(rest) = file.relative_path.strip_prefix("crates/") {
                    if let Some((crate_name, _)) = rest.split_once('/') {
                        *locs.entry(crate_name.to_string()).or_insert(0) += file.line_count;
                    }
                }
            }
            for (name, loc) in locs {
                if loc as u64 > BUDGET {
                    violations.push(AuditViolation {
                        code: "CYNIC-ARCH-005".to_string(),
                        message: format!(
                            "God crate budget exceeded: `{}` has {} LOC (limit {}).",
                            name, loc, BUDGET
                        ),
                        severity: AuditSeverity::Warning,
                        category: AuditCategory::Boundary,
                        span: None,
                        remediation: Some(
                            "Extract domain verticals into dedicated DDD crates to bring the crate under the 45,000 LOC budget.".to_string(),
                        ),
                    });
                }
            }
        }

        violations
    }
}

impl AuditRule for GodCrateBudgetExceededRule {
    fn spec(&self) -> AuditRuleSpec {
        GodCrateBudgetRule.spec()
    }
    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        GodCrateBudgetRule.evaluate(index)
    }
}

// ---------------------------------------------------------------------------
// CYNIC-ARCH-006 — UnnormedVocabularyDrift
// ---------------------------------------------------------------------------

const UNNORMED_TERMS: &[&str] = &["god_object", "shim_layer", "god-object", "shim-layer"];

pub struct UnnormedVocabularyDriftRule;
pub struct VocabularyDriftRule;
pub type Arch006Rule = UnnormedVocabularyDriftRule;

impl UnnormedVocabularyDriftRule {
    fn banned_terms(index: &WorkspaceSourceIndex) -> Vec<String> {
        let mut terms: Vec<String> = UNNORMED_TERMS.iter().map(|s| s.to_string()).collect();
        for file in &index.json_files {
            if !(file.relative_path == "contracts/glossary.json"
                || file.relative_path.ends_with("glossary.json"))
            {
                continue;
            }
            if let Ok(doc) = serde_json::from_str::<Value>(&file.content) {
                if let Some(terms_obj) = doc.get("terms").and_then(|v| v.as_object()) {
                    for (_key, body) in terms_obj {
                        if let Some(arr) = body.get("banned_synonyms").and_then(|v| v.as_array()) {
                            for item in arr {
                                if let Some(s) = item.as_str() {
                                    terms.push(s.to_string());
                                }
                            }
                        }
                    }
                }
                if let Some(arr) = doc.get("banned_terms").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            terms.push(s.to_string());
                        }
                    }
                }
            }
        }
        terms.sort();
        terms.dedup();
        terms
    }
}

impl AuditRule for UnnormedVocabularyDriftRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "CYNIC-ARCH-006".to_string(),
            name: "unnormed_vocabulary_drift".to_string(),
            description: "Code identifiers and docs must use canonical glossary terms, not banned jargon.".to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Warning,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let terms = Self::banned_terms(index);
        let mut violations = Vec::new();

        let all_files: Vec<&_> = index
            .rust_files
            .iter()
            .chain(index.json_files.iter())
            .chain(index.manifests.iter())
            .collect();

        for file in all_files {
            if file.relative_path.contains("audit_engine")
                || file.relative_path.contains("audit-engine")
                || file.relative_path.contains("/tests/")
                || file.relative_path.contains("_test.rs")
            {
                continue;
            }
            let lower = file.content.to_lowercase();
            for term in &terms {
                if lower.contains(&term.to_lowercase()) {
                    violations.push(AuditViolation {
                        code: "CYNIC-ARCH-006".to_string(),
                        message: format!(
                            "Unnormed vocabulary drift: banned term `{}` found in `{}`.",
                            term, file.relative_path
                        ),
                        severity: AuditSeverity::Warning,
                        category: AuditCategory::Boundary,
                        span: None,
                        remediation: Some(
                            "Use canonical glossary terms instead of banned jargon.".to_string(),
                        ),
                    });
                    break;
                }
            }
        }

        violations
    }
}

impl AuditRule for VocabularyDriftRule {
    fn spec(&self) -> AuditRuleSpec {
        UnnormedVocabularyDriftRule.spec()
    }
    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        UnnormedVocabularyDriftRule.evaluate(index)
    }
}
