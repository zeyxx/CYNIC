//! `NAM-001` — the workspace must not name its concepts with biological metaphors.
//!
//! CYNIC's domain language was borrowed from biology: components were
//! "organs", axiom scorers were "dogs", the assembled system was an
//! "organism". The metaphor was not free: it hid the actual mechanism
//! behind a vivid but non-technical word, and it leaked into the persisted
//! schema, where it became a storage contract nobody could rename without
//! a data migration.
//!
//! The canonical vocabulary lives in `cynic_protocol::vocabulary`. This rule
//! is the ratchet.
//!
//! ## Why a ledger and not one finding per line
//!
//! The migration is a BACKLOG, not a defect per occurrence. Reporting 1024
//! findings would bury the signal: every finding would be "the same known
//! problem, in another file", and a reader who cannot tell a known problem
//! from a new one learns to ignore the rule entirely. A rule nobody reads is
//! a rule that does not hold.
//!
//! So the rule counts occurrences per file and compares that count against a
//! declared baseline, exactly like `ARCH-005` compares module inversions
//! against `infra/module-inversion-ledger.json`:
//!
//! - an occurrence in a file with no baseline entry is `NAM-001-NEW-METAPHOR`
//!   — new debt, fails the gate;
//! - a file whose count grew above its baseline is `NAM-001-METAPHOR-GREW`
//!   — the baseline may only shrink;
//! - a baseline entry with no observed occurrence is `NAM-001-STALE-ENTRY`
//!   — paid debt left in the list hides that the list is stale.
//!
//! The baseline lives in `infra/retired-metaphor-ledger.json`.
//!
//! ## What is NOT detected
//!
//! Two words look biological but are infrastructure identifiers. Renaming
//! them would break live DNS and host addressing, so they are exempt by
//! construction, not by suppression:
//!
//! - `service` — the service network plane (`*.service.cynic`,
//!   `network_policy.planes.service`). Its canonical name is `service`;
//!   `service` is the legacy registry key handled by
//!   `cynic_network::plane_vocabulary`.
//! - `cortex` — the operator workstation hostname (`cynic-host-cortex`).
//!
//! Ordinary English words that happen to be anatomical are also exempt:
//! `symptom` (a failure symptom), `diagnosis`, `triage`, `absorb`,
//! `skeleton` (a CLI skeleton). Only the terms the vocabulary actually lists
//! are retired.

use std::collections::BTreeMap;
use std::path::Path;

use cynic_protocol::vocabulary::{
    INFRASTRUCTURE_COMPOSED_IDENTIFIERS, RETIRED_METAPHORS,
};
use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

const RULE_ID: &str = "NAM-001";
/// The baseline the rule compares the observation against.
///
/// Public because the ratchet test reads the same file: a test that read a
/// DIFFERENT path than the rule would compare two different baselines and
/// agree with neither.
pub const LEDGER_PATH: &str = "infra/retired-metaphor-ledger.json";

/// `NAM-001` — retired biological metaphors must not reappear.
pub struct RetiredMetaphorRule;

impl AuditRule for RetiredMetaphorRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: RULE_ID.to_string(),
            name: "retired_biological_metaphor".to_string(),
            description: "Domain concepts must be named with technical terms, not biological \
                 metaphors. A component is a 'component', an axiom scorer is an \
                 'evaluator', the running system is a 'deployment'. 'synapse' (the \
                 service network plane) and 'cortex' (the operator workstation) are \
                 infrastructure identifiers, not metaphors, and are exempt. Declared \
                 occurrences live in infra/retired-metaphor-ledger.json and the baseline \
                 may only shrink."
                .to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Debt,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        let observed = observe(index);

        // The workspace root, recovered from any source file: every path in
        // the ledger is repo-relative, so walk up from the first one.
        let Some(root) = index.rust_files.first().map(|f| {
            let mut path = f.absolute_path.clone();
            for _ in 0..f.relative_path.matches('/').count() + 1 {
                path.pop();
            }
            path
        }) else {
            return violations;
        };

        let Some(declared) = declared_counts(&root) else {
            violations.push(AuditViolation {
                code: format!("{RULE_ID}-LEDGER-UNREADABLE"),
                message: format!(
                    "{LEDGER_PATH} could not be read or parsed; the retired metaphor \
                     comparator could not run. This is an explicit unknown, not a pass."
                ),
                severity: AuditSeverity::Critical,
                category: AuditCategory::ResidualUnknown,
                span: None,
                remediation: Some(format!("Restore {LEDGER_PATH} as valid JSON.")),
            });
            return violations;
        };

        for (path, (count, evidence, terms)) in &observed {
            let terms = terms.join(", ");
            match declared.get(path) {
                None => violations.push(AuditViolation {
                    code: format!("{RULE_ID}-NEW-METAPHOR"),
                    message: format!(
                        "Retired biological metaphor in {path} ({count} occurrence(s), \
                         first at {evidence}): {terms}. Use the canonical term instead: \
                         {}.",
                        canonical_hint()
                    ),
                    severity: AuditSeverity::Debt,
                    category: AuditCategory::Boundary,
                    span: Some(AuditSpan {
                        file_path: path.clone(),
                        line_start: 0,
                        line_end: 0,
                        snippet: Some(evidence.clone()),
                    }),
                    remediation: Some(format!(
                        "Rename to the canonical term, or record the file deliberately in \
                         {LEDGER_PATH}."
                    )),
                }),
                Some(&allowed) if count > &allowed => violations.push(AuditViolation {
                    code: format!("{RULE_ID}-METAPHOR-GREW"),
                    message: format!(
                        "Retired biological metaphor in {path} was recorded {allowed}x and is \
                         now observed {count}x ({terms}). The baseline may only shrink."
                    ),
                    severity: AuditSeverity::Debt,
                    category: AuditCategory::Boundary,
                    span: Some(AuditSpan {
                        file_path: path.clone(),
                        line_start: 0,
                        line_end: 0,
                        snippet: Some(evidence.clone()),
                    }),
                    remediation: Some(format!(
                        "Remove the new occurrence, or lower the count in {LEDGER_PATH} if \
                         the count is correct."
                    )),
                }),
                Some(_) => {}
            }
        }

        for (path, allowed) in &declared {
            if !observed.contains_key(path) {
                violations.push(AuditViolation {
                    code: format!("{RULE_ID}-STALE-ENTRY"),
                    message: format!(
                        "{LEDGER_PATH} records {path} ({allowed} occurrence(s)) but none was \
                         observed. Paid debt left in the list hides that the list is stale."
                    ),
                    severity: AuditSeverity::Debt,
                    category: AuditCategory::Boundary,
                    span: None,
                    remediation: Some(format!("Delete the `{path}` entry from {LEDGER_PATH}.")),
                });
            }
        }

        violations
    }
}

/// The observed retired-metaphor occurrences, per file.
///
/// This is the single source of truth for both the rule and the ledger
/// generator: a baseline recorded from a DIFFERENT observation than the one
/// the rule compares against is a baseline that fails for reasons that have
/// nothing to do with the vocabulary.
///
/// Each entry is `(count, first evidence, terms seen)`.
pub fn observe(index: &WorkspaceSourceIndex) -> BTreeMap<String, (usize, String, Vec<String>)> {
    let mut observed: BTreeMap<String, (usize, String, Vec<String>)> = BTreeMap::new();

    let all_files = index
        .rust_files
        .iter()
        .chain(index.json_files.iter())
        .chain(index.manifests.iter());

    for file in all_files {
        // The rule may not report its own detector: these files name the
        // retired terms in order to find them. Keyed on the FILE NAME, so
        // the next extraction does not silently re-arm the rule.
        if file.relative_path.ends_with("retired_metaphor_rule.rs")
            || file.relative_path.ends_with("vocabulary.rs")
            || file.relative_path.ends_with("retired-metaphor-ledger.json")
            || file.relative_path.ends_with("examples/nam_ledger.rs")
        {
            continue;
        }

        // Test fixtures and recorded evidence legitimately name the old
        // terms: a migration fixture asserts the OLD key is still read,
        // and a runtime checkpoint records what was observed. Excluding
        // them keeps the rule actionable instead of noisy.
        if file.relative_path.contains("/tests/")
            || file.relative_path.starts_with("tests/")
            || file.relative_path.contains("runtime-checkpoints")
            || file.relative_path.contains("retrieval_training_pairs")
        {
            continue;
        }

        for (idx, line) in file.content.lines().enumerate() {
            for term in retired_terms_in_line(line) {
                let entry = observed.entry(file.relative_path.clone()).or_insert((
                    0,
                    format!("{}:{}", file.relative_path, idx + 1),
                    Vec::new(),
                ));
                entry.0 += 1;
                if !entry.2.iter().any(|seen| seen == &term) {
                    entry.2.push(term);
                }
            }
        }
    }

    observed
}

/// The declared baseline: file path -> occurrence count.
fn declared_counts(root: &Path) -> Option<BTreeMap<String, usize>> {
    let text = std::fs::read_to_string(root.join(LEDGER_PATH)).ok()?;
    let document: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut out = BTreeMap::new();
    for entry in document.get("entries")?.as_array()? {
        out.insert(
            entry.get("path")?.as_str()?.to_string(),
            entry.get("count")?.as_u64()? as usize,
        );
    }
    Some(out)
}

/// The canonical replacement for each retired term, as one line.
fn canonical_hint() -> String {
    RETIRED_METAPHORS
        .iter()
        .map(|mapping| format!("{} -> {}", mapping.retired, mapping.canonical))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every retired term present in one line, with the spelling as written.
///
/// The line is split into identifier parts (camelCase, snake_case,
/// kebab-case, dotted) and into whole words for prose. Both surfaces carry
/// the metaphor, and a rule that only scanned one of them would let the
/// other rot.
fn retired_terms_in_line(line: &str) -> Vec<String> {
    // A line that names a known infrastructure identifier is exempt. The
    // name is a NAME: `organ-headscale` is a running machine, and the
    // segment `organ` inside it is part of the address, not a metaphor.
    // The exemption is by WHOLE identifier, so an identifier that merely
    // starts with a known name is still reported.
    if line_contains_infrastructure_identifier(line) {
        return Vec::new();
    }

    let mut found = Vec::new();

    // Identifier parts: split on separators and case boundaries.
    for part in split_identifier_parts(line) {
        if is_retired_word(&part) {
            found.push(part);
        }
    }

    // Prose words: the line stripped of identifier punctuation, split on
    // whitespace. This is what catches "an Organ is silent".
    for word in line
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
    {
        if is_retired_word(word) && !found.iter().any(|existing| existing == word) {
            found.push(word.to_string());
        }
    }

    found
}

/// True when the line names a known infrastructure identifier.
///
/// The identifier is matched as written — including its separators — so
/// `organ-headscale` is found in prose, in a string literal, and in a
/// comment alike.
fn line_contains_infrastructure_identifier(line: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    INFRASTRUCTURE_COMPOSED_IDENTIFIERS
        .iter()
        .any(|known| lowered.contains(*known))
}

/// Split a line into identifier-sized parts.
///
/// `OrganRemediation` yields `Organ`, `Remediation`; `organ_silence`
/// yields `organ`, `silence`; `dog-id` yields `dog`, `id`.
///
/// Every non-alphanumeric character is a separator, not just `_` and `-`.
/// A source line is full of punctuation — `"organ_silence"`, `kind =`,
/// `dogs: Vec<..>` — and a separator set that forgets the quotes and the
/// brackets yields the part `"organ` with a leading quote, which then
/// matches no term and the detection silently fails on exactly the lines
/// it was written for.
fn split_identifier_parts(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut previous_was_lower = false;

    for character in line.chars() {
        if !character.is_alphanumeric() {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            previous_was_lower = false;
            continue;
        }

        if character.is_uppercase() && previous_was_lower {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
        }

        previous_was_lower = character.is_lowercase() || character.is_numeric();
        current.push(character);
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

/// True when one word is a retired term.
///
/// The vocabulary owns the list, including the plural forms. The
/// infrastructure identifiers `service` and `cortex` are not in the
/// vocabulary at all, so they are exempt by construction: adding them to
/// the retired list is the only way to make them a violation, and they are
/// deliberately absent.
fn is_retired_word(word: &str) -> bool {
    cynic_protocol::vocabulary::is_retired_term(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms_in(line: &str) -> Vec<String> {
        retired_terms_in_line(line)
    }

    #[test]
    fn an_organ_identifier_is_detected() {
        let found = terms_in("pub struct OrganRemediation { }");
        assert!(
            found.iter().any(|term| term == "Organ"),
            "expected Organ in {:?}",
            found
        );
    }

    #[test]
    fn a_snake_case_organ_identifier_is_detected() {
        let found = terms_in("let kind = \"organ_silence\";");
        assert!(
            found.iter().any(|term| term == "organ"),
            "expected organ in {:?}",
            found
        );
    }

    #[test]
    fn a_plural_dog_identifier_is_detected() {
        let found = terms_in("pub failed_dogs: Vec<String>,");
        assert!(
            found.iter().any(|term| term == "dogs"),
            "expected dogs in {:?}",
            found
        );
    }

    /// A real machine name is a name, not a metaphor. `organ-headscale` is
    /// a running container on the Proxmox hypervisor, and the audit must
    /// not report it — but the exemption is by WHOLE identifier, so a
    /// near-miss is still reported.
    #[test]
    fn an_infrastructure_identifier_is_not_a_metaphor() {
        assert!(terms_in("organ-headscale holds 10.10.10.40 on the canonical plane").is_empty());
        assert!(terms_in("let host = \"organ-forgejo\";").is_empty());
        assert!(terms_in("/// organ-surrealdb holds the live database").is_empty());
    }

    /// The exemption must not become a blanket: a bare `organ` is still
    /// retired, and an identifier that merely starts with a known name is
    /// still reported.
    #[test]
    fn the_infrastructure_exemption_is_not_a_blanket() {
        assert!(
            terms_in("let organ = 1;").iter().any(|term| term == "organ"),
            "a bare organ must still be reported"
        );
        assert!(
            terms_in("let organ_headscale_extra = 1;")
                .iter()
                .any(|term| term == "organ"),
            "a near-miss of a known name must still be reported"
        );
    }

    #[test]
    fn prose_naming_the_concept_is_detected() {
        let found = terms_in("// The Organ is silent for 30s");
        assert!(
            found.iter().any(|term| term.eq_ignore_ascii_case("organ")),
            "expected organ in {:?}",
            found
        );
    }

    /// The infrastructure identifiers must never be findings: `service` is
    /// the live service network plane and `cortex` is the operator
    /// workstation hostname. A rule that flagged them would send the next
    /// agent to rename live DNS.
    #[test]
    fn infrastructure_identifiers_are_not_findings() {
        assert!(terms_in("let plane = \"synapse\";").is_empty());
        assert!(terms_in("const BUILDER: &str = \"cynic-host-cortex\";").is_empty());
        assert!(terms_in("network_policy.planes.synapse.dns_zone").is_empty());
    }

    /// Ordinary English words that are also anatomical are not the retired
    /// vocabulary and must pass.
    #[test]
    fn ordinary_english_anatomical_words_are_not_findings() {
        assert!(terms_in("let symptom = report.error;").is_empty());
        assert!(terms_in("diagnosis of the failure").is_empty());
        assert!(terms_in("triage branches by measurement").is_empty());
    }

    /// A word that merely CONTAINS a retired term is not a violation:
    /// `catalog` and `dogma` are technical words.
    #[test]
    fn words_containing_a_retired_term_are_not_findings() {
        assert!(terms_in("let catalog = load();").is_empty());
        assert!(terms_in("/// dogma of the quorum").is_empty());
    }

    #[test]
    fn a_canonical_term_is_never_a_finding() {
        assert!(terms_in("pub struct ComponentRemediation { }").is_empty());
        assert!(terms_in("pub failed_evaluators: Vec<String>,").is_empty());
    }
}
