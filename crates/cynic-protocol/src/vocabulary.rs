//! Canonical technical vocabulary — the single translation point for the
//! retired biological metaphors.
//!
//! ## Why this exists
//!
//! CYNIC's domain language was borrowed from biology: components were
//! "organs", evaluators were "dogs", the whole system was an "organism".
//! The metaphors cost real engineering:
//!
//! - they hid the actual mechanism behind a vivid but non-technical word
//!   (a "dog" is an evaluator that scores axioms and joins a quorum; an
//!   "organ" is a supervised component that emits health signals);
//! - they leaked into the persisted schema (`dog_usage`, `dog_stats`,
//!   `organs`, `organ_audits`), so the metaphor became a storage contract
//!   nobody could rename without a data migration;
//! - they made the codebase read as folklore instead of as a specification.
//!
//! ## The vocabulary
//!
//! | Retired term | Canonical term | What it actually is |
//! |--------------|----------------|---------------------|
//! | `organ`     | `component`    | a supervised runtime component |
//! | `dog`       | `evaluator`    | an axiom scorer in a quorum |
//! | `organism`  | `deployment`   | the assembled running system |
//! | `organic`   | `candidate`    | an unclaimed opportunity |
//!
//! ## What this module does NOT rename
//!
//! Two words look biological but are infrastructure identifiers, and
//! renaming them would break live DNS and host addressing:
//!
//! - `synapse` — the service network plane (`*.synapse.cynic`,
//!   `network_policy.planes.synapse`). The canonical plane name is
//!   `service`; `synapse` is the legacy registry key, handled by
//!   `cynic_network::plane_vocabulary`.
//! - `cortex` — the operator workstation hostname (`cynic-host-cortex`).
//!
//! ## How to use it
//!
//! New code must not write the retired terms. The audit rule
//! `NAM-001` in `cynic-audit-engine` fails the build when one reappears.
//! When an existing name must be read from a persisted artifact or a
//! legacy config, use the read helpers here so the mapping stays in one
//! place and the compatibility surface is visible.

/// A retired biological term and its canonical replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabularyMapping {
    /// The retired term, lowercase.
    pub retired: &'static str,
    /// The canonical technical term, lowercase.
    pub canonical: &'static str,
    /// What the concept actually is, for the audit remediation text.
    pub meaning: &'static str,
}

/// The full retired-metaphor vocabulary.
///
/// Ordered by decreasing blast radius so a reader sees the expensive
/// renames first.
pub const RETIRED_METAPHORS: &[VocabularyMapping] = &[
    VocabularyMapping {
        retired: "organ",
        canonical: "component",
        meaning: "a supervised runtime component that emits health signals",
    },
    VocabularyMapping {
        retired: "dog",
        canonical: "evaluator",
        meaning: "an axiom scorer that contributes to a quorum",
    },
    VocabularyMapping {
        retired: "organism",
        canonical: "deployment",
        meaning: "the assembled running system",
    },
    VocabularyMapping {
        retired: "organic",
        canonical: "candidate",
        meaning: "an observed, unclaimed opportunity",
    },
];

/// True when `word` is a retired term.
///
/// The comparison is case-insensitive and whole-word, so `dogma` and
/// `organise` are not false positives. Plurals are part of the term:
/// `dogs` is the plural of the retired `dog` and must be caught too,
/// because the plural is the form that appears most often in code
/// (`failed_dogs`, `participating_dogs_count`).
pub fn is_retired_term(word: &str) -> bool {
    let lowered = word.to_ascii_lowercase();
    RETIRED_METAPHORS
        .iter()
        .any(|mapping| lowered == mapping.retired || lowered == plural_of(mapping.retired))
}

/// The regular English plural of a term: `dog` -> `dogs`, `organism` ->
/// `organisms`.
///
/// Only the `-s` form is generated. The irregular plurals that matter here
/// (`organ` -> `organs`) are also `-s`; anything else would be a term the
/// vocabulary does not list.
fn plural_of(term: &str) -> String {
    format!("{term}s")
}

/// The canonical replacement for a retired term, if there is one.
pub fn canonical_term(word: &str) -> Option<&'static str> {
    let lowered = word.to_ascii_lowercase();
    RETIRED_METAPHORS
        .iter()
        .find(|mapping| lowered == mapping.retired)
        .map(|mapping| mapping.canonical)
}

/// The canonical replacement for a persisted key.
///
/// Persisted artifacts written before the migration still carry the old
/// key. This is the ONE place that knows how to read them, so the
/// compatibility surface is bounded and greppable instead of spread over
/// every reader.
pub fn canonical_persisted_key(key: &str) -> &str {
    match key {
        "organs" => "components",
        "organ_audits" => "component_audits",
        "organ" => "component",
        "organ_id" => "component_id",
        "dogs" => "evaluators",
        "dog" => "evaluator",
        "dog_id" => "evaluator_id",
        "dog_count" => "evaluator_count",
        "dog_usage" => "evaluator_usage",
        "dog_stats" => "evaluator_stats",
        "organism" => "deployment",
        "organic_opportunities" => "candidates",
        "organic_opportunity" => "candidate",
        other => other,
    }
}

/// True when the key names a persisted artifact that must be read through
/// [`canonical_persisted_key`] rather than renamed in place.
pub fn is_legacy_persisted_key(key: &str) -> bool {
    canonical_persisted_key(key) != key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_retired_term_maps_to_a_canonical_term() {
        for mapping in RETIRED_METAPHORS {
            assert_eq!(
                canonical_term(mapping.retired),
                Some(mapping.canonical),
                "{} must map to {}",
                mapping.retired,
                mapping.canonical
            );
        }
    }

    #[test]
    fn retired_term_detection_is_case_insensitive() {
        assert!(is_retired_term("organ"));
        assert!(is_retired_term("Organ"));
        assert!(is_retired_term("ORGAN"));
        assert!(is_retired_term("dog"));
        assert!(is_retired_term("Dogs"));
    }

    /// The plural is the form that dominates the codebase
    /// (`failed_dogs`, `participating_dogs_count`, `organs`), so a rule
    /// that only catches the singular would pass the workspace while the
    /// metaphor is still everywhere.
    #[test]
    fn retired_term_detection_covers_plurals() {
        assert!(is_retired_term("dogs"));
        assert!(is_retired_term("Dogs"));
        assert!(is_retired_term("organs"));
        assert!(is_retired_term("organisms"));
    }

    #[test]
    fn retired_term_detection_is_whole_word() {
        // A technical word that merely contains a retired term is fine.
        assert!(!is_retired_term("dogma"));
        assert!(!is_retired_term("organise"));
        assert!(!is_retired_term("catalog"));
        assert!(!is_retired_term("synapse"));
        assert!(!is_retired_term("cortex"));
    }

    #[test]
    fn canonical_terms_are_not_retired_terms() {
        for mapping in RETIRED_METAPHORS {
            assert!(
                !is_retired_term(mapping.canonical),
                "the canonical term {} must not itself be retired",
                mapping.canonical
            );
        }
    }

    #[test]
    fn persisted_keys_translate_to_canonical_names() {
        assert_eq!(canonical_persisted_key("organs"), "components");
        assert_eq!(canonical_persisted_key("dogs"), "evaluators");
        assert_eq!(canonical_persisted_key("dog_id"), "evaluator_id");
        assert_eq!(
            canonical_persisted_key("organic_opportunities"),
            "candidates"
        );
    }

    #[test]
    fn unknown_keys_pass_through_unchanged() {
        assert_eq!(canonical_persisted_key("dogma"), "dogma");
        assert_eq!(canonical_persisted_key("components"), "components");
        assert!(!is_legacy_persisted_key("components"));
        assert!(is_legacy_persisted_key("organs"));
    }
}
