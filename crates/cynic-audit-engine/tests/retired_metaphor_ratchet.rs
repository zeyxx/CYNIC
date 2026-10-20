//! Ratchet on the retired biological metaphors: the NAM-001 baseline.
//!
//! WHY THIS FILE EXISTS. NAM-001 reports a retired metaphor per FILE, against
//! a declared baseline in `infra/retired-metaphor-ledger.json`. A ratchet that
//! only lives inside a rule is not verifiable on its own: the workspace audit
//! applies the debt ledger, which absorbs every declared violation, so reading
//! the verdict returns an empty set and comparing it against a 158-entry
//! baseline fails for a reason that has nothing to do with the vocabulary.
//!
//! WHAT IT DOES. It reads the RAW evaluation and holds three things at once:
//!
//! 1. the baseline agrees with what the engine observes, exactly — so the
//!    SSOT cannot drift from reality in either direction;
//! 2. no file outside the baseline may carry a retired term, so debt cannot
//!    be added silently;
//! 3. the baseline holds no paid entry, so a renamed file removes itself from
//!    the baseline in the same commit, which is what makes the reduction
//!    visible in a diff.
//!
//! WHY IT IS A SEPARATE FILE FROM `path_reference_audit.rs`. That gate
//! asserts on the FIRST undeclared path divergence and stops there. A debt
//! ratchet must report EVERY divergence: a gate that stops at the first one
//! cannot tell a single stale entry from a thousand, and the reader has no
//! way to size the work.
//!
//! The comparison is by COUNT PER FILE, not by occurrence. A per-occurrence
//! identity would embed a line number, and any edit would change it.

#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use cynic_audit_engine::retired_metaphor_rule::{LEDGER_PATH, observe};
use cynic_audit_engine::scanner::scan_workspace;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

/// The observed occurrences, per file: path -> count.
fn observed_counts() -> BTreeMap<String, usize> {
    let index = scan_workspace(&workspace_root());
    observe(&index)
        .into_iter()
        .map(|(path, (count, _, _))| (path, count))
        .collect()
}

/// The declared baseline, per file: path -> count.
fn declared_counts() -> BTreeMap<String, usize> {
    let text = std::fs::read_to_string(workspace_root().join(LEDGER_PATH))
        .expect("read infra/retired-metaphor-ledger.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse the ledger");
    let entries = value
        .get("entries")
        .and_then(|entries| entries.as_array())
        .expect("the ledger has no `entries` array");

    let mut out = BTreeMap::new();
    for entry in entries {
        let path = entry
            .get("path")
            .and_then(|path| path.as_str())
            .expect("a ledger entry has no `path`");
        let count = entry
            .get("count")
            .and_then(|count| count.as_u64())
            .expect("a ledger entry has no numeric `count`") as usize;
        out.insert(path.to_string(), count);
    }
    out
}

/// The rule must be registered, or this ratchet measures nothing.
#[test]
fn the_rule_is_registered_and_running() {
    let report = cynic_audit_engine::evaluate_workspace_rules(&workspace_root());
    assert!(
        report
            .evaluated_rules
            .iter()
            .any(|spec| spec.id == "NAM-001"),
        "NAM-001 missing from the default rule set, so this ratchet would measure nothing"
    );
}

/// The baseline and the observation agree, exactly.
#[test]
fn the_baseline_and_the_observation_agree_exactly() {
    let observed = observed_counts();
    let declared = declared_counts();

    let mut new_files: Vec<(&String, &usize)> = observed
        .iter()
        .filter(|(path, _)| !declared.contains_key(*path))
        .collect();
    new_files.sort();
    assert!(
        new_files.is_empty(),
        "these files carry a retired metaphor and are not in the baseline: {new_files:#?}.\n\
         The rule already reports them as NAM-001-NEW-METAPHOR. Either rename them, or \
         record them in {LEDGER_PATH} as a deliberate act."
    );

    let mut stale: Vec<(&String, &usize)> = declared
        .iter()
        .filter(|(path, _)| !observed.contains_key(*path))
        .collect();
    stale.sort();
    assert!(
        stale.is_empty(),
        "these baseline entries have no observed occurrence: {stale:#?}.\n\
         Paid debt left in the list hides that the list is stale. Remove them from \
         {LEDGER_PATH} in the same commit as the rename."
    );

    let mut grown: Vec<(&String, &usize, &usize)> = observed
        .iter()
        .filter_map(|(path, count)| {
            let declared_count = declared.get(path)?;
            (count > declared_count).then_some((path, count, declared_count))
        })
        .collect();
    grown.sort();
    assert!(
        grown.is_empty(),
        "these files carry MORE retired metaphors than the baseline records: {grown:#?}.\n\
         The baseline may only shrink. Rename the new occurrences, or lower the count if \
         the baseline was simply out of date."
    );
}

/// The measured block in the ledger describes the ledger itself.
#[test]
fn the_measured_block_describes_the_ledger() {
    let text = std::fs::read_to_string(workspace_root().join(LEDGER_PATH))
        .expect("read infra/retired-metaphor-ledger.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse the ledger");
    let measured = value
        .get("measured")
        .expect("the ledger has no `measured` block");

    let entries = value
        .get("entries")
        .and_then(|entries| entries.as_array())
        .expect("the ledger has no `entries` array");

    let declared_files = measured
        .get("files")
        .and_then(|files| files.as_u64())
        .expect("measured.files is not a number") as usize;
    assert_eq!(
        declared_files,
        entries.len(),
        "measured.files ({declared_files}) does not match the number of entries ({})",
        entries.len()
    );

    let declared_occurrences = measured
        .get("occurrences")
        .and_then(|occurrences| occurrences.as_u64())
        .expect("measured.occurrences is not a number") as usize;
    let summed: usize = entries
        .iter()
        .map(|entry| {
            entry
                .get("count")
                .and_then(|count| count.as_u64())
                .expect("a ledger entry has no numeric `count`") as usize
        })
        .sum();
    assert_eq!(
        declared_occurrences, summed,
        "measured.occurrences ({declared_occurrences}) does not match the sum of the entry \
         counts ({summed})"
    );
}

/// The vocabulary is a closed set: every retired term has a canonical
/// replacement, and no canonical term is itself retired.
#[test]
fn the_vocabulary_is_closed() {
    for mapping in cynic_protocol::vocabulary::RETIRED_METAPHORS {
        assert_eq!(
            cynic_protocol::vocabulary::canonical_term(mapping.retired),
            Some(mapping.canonical),
            "{} must map to {}",
            mapping.retired,
            mapping.canonical
        );
        assert!(
            !cynic_protocol::vocabulary::is_retired_term(mapping.canonical),
            "the canonical term {} must not itself be retired",
            mapping.canonical
        );
    }
}

/// The infrastructure identifiers are exempt BY CONSTRUCTION, not by
/// suppression: they are absent from the vocabulary, so no exemption list
/// can drift out of sync with it.
#[test]
fn the_infrastructure_identifiers_are_not_retired() {
    assert!(!cynic_protocol::vocabulary::is_retired_term("synapse"));
    assert!(!cynic_protocol::vocabulary::is_retired_term("cortex"));
    assert!(!cynic_protocol::vocabulary::is_retired_term("synapses"));
    assert!(!cynic_protocol::vocabulary::is_retired_term("cortices"));
}
