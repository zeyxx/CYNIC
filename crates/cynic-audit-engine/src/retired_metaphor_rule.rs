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
//! is the ratchet: it fails the build when a retired term reappears.
//!
//! ## What is detected
//!
//! Two surfaces, because the metaphor hides in both:
//!
//! - **identifiers** — `OrganRemediation`, `dog_id`, `organ_silence`,
//!   `failed_dogs`. A camel/snake identifier is split on its case and
//!   separator boundaries, and each part is checked against the vocabulary.
//! - **prose** — comments, doc comments, and string literals naming the
//!   concept ("an Organ is silent", "the organism state").
//!
//! ## What is NOT detected
//!
//! Two words look biological but are infrastructure identifiers. Renaming
//! them would break live DNS and host addressing, so they are exempt by
//! construction, not by suppression:
//!
//! - `synapse` — the service network plane (`*.synapse.cynic`,
//!   `network_policy.planes.synapse`). Its canonical name is `service`;
//!   `synapse` is the legacy registry key handled by
//!   `cynic_network::plane_vocabulary`.
//! - `cortex` — the operator workstation hostname (`cynic-host-cortex`).
//!
//! Ordinary English words that happen to be anatomical are also exempt:
//! `symptom` (a failure symptom), `diagnosis`, `triage`, `absorb`,
//! `skeleton` (a CLI skeleton), `muscle` (prose only, not an identifier).
//! Only the terms the vocabulary actually lists are retired.

use cynic_protocol::vocabulary::{RETIRED_METAPHORS, canonical_term};
use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

/// `NAM-001` — retired biological metaphors must not reappear.
pub struct RetiredMetaphorRule;

impl AuditRule for RetiredMetaphorRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "NAM-001".to_string(),
            name: "retired_biological_metaphor".to_string(),
            description: "Domain concepts must be named with technical terms, not biological \
                 metaphors. A component is a 'component', an axiom scorer is an \
                 'evaluator', the running system is a 'deployment'. 'synapse' (the \
                 service network plane) and 'cortex' (the operator workstation) are \
                 infrastructure identifiers, not metaphors, and are exempt."
                .to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Debt,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        let all_files = index
            .rust_files
            .iter()
            .chain(index.json_files.iter())
            .chain(index.manifests.iter());

        for file in all_files {
            // The rule may not report its own detector: this file names the
            // retired terms in order to find them. Keyed on the FILE NAME, so
            // the next extraction does not silently re-arm the rule.
            if file.relative_path.ends_with("retired_metaphor_rule.rs")
                || file.relative_path.ends_with("vocabulary.rs")
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
                    let Some(canonical) = canonical_term(&term) else {
                        continue;
                    };
                    let meaning = RETIRED_METAPHORS
                        .iter()
                        .find(|mapping| mapping.retired == term.to_ascii_lowercase())
                        .map(|mapping| mapping.meaning)
                        .unwrap_or("a technical concept");

                    violations.push(AuditViolation {
                        code: "NAM-001-RETIRED-METAPHOR".to_string(),
                        message: format!(
                            "Retired biological metaphor '{}' in {}: use '{}' ({}).",
                            term, file.relative_path, canonical, meaning
                        ),
                        severity: AuditSeverity::Debt,
                        category: AuditCategory::Boundary,
                        span: Some(AuditSpan {
                            file_path: file.relative_path.clone(),
                            line_start: idx + 1,
                            line_end: idx + 1,
                            snippet: Some(line.trim().to_string()),
                        }),
                        remediation: Some(format!(
                            "Rename to '{}'. The vocabulary is \
                             cynic_protocol::vocabulary.",
                            canonical
                        )),
                    });
                }
            }
        }

        violations
    }
}

/// Every retired term present in one line, with the spelling as written.
///
/// The line is split into identifier parts (camelCase, snake_case,
/// kebab-case, dotted) and into whole words for prose. Both surfaces carry
/// the metaphor, and a rule that only scanned one of them would let the
/// other rot.
fn retired_terms_in_line(line: &str) -> Vec<String> {
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
/// infrastructure identifiers `synapse` and `cortex` are not in the
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

    #[test]
    fn prose_naming_the_concept_is_detected() {
        let found = terms_in("// The Organ is silent for 30s");
        assert!(
            found.iter().any(|term| term.eq_ignore_ascii_case("organ")),
            "expected organ in {:?}",
            found
        );
    }

    /// The infrastructure identifiers must never be findings: `synapse` is
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
