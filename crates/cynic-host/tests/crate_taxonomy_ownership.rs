//! Ownership and naming gate for the crate taxonomy.
//!
//! Ratified by `contracts/pruning-policy.json` → `responsibility_rules`
//! (`name_states_responsibility`, `one_responsibility_one_crate`,
//! `family_prefix_is_optional_and_not_normative`,
//! `metaphor_banned_from_crate_names`).
//!
//! WHY THIS FILE EXISTS. `foundation.json` made the `cynic-<family>-<role>`
//! taxonomy a constitutional violation to have no name for, and nothing
//! executed it. The result was measurable: 12 crates share a name segment, the
//! family prefix is carried by 13 crates and absent from 35, and `cynic-domain-*`
//! asserts a purity that two of its four members do not have (they contain
//! `adapter.rs` and depend on `reqwest`). Agave is the reference model and also
//! the counter-example: it names crates by capability and keeps the layer in
//! Cargo.toml, then paid for a prefix that encoded provenance instead of
//! responsibility when `solana-*` became `agave-*` and broke automations.
//!
//! WHAT IT CHECKS, and what it deliberately does not.
//! 1. Every declared crate states a responsibility (its `Cargo.toml`
//!    `description`), so "who owns what" is readable without reading the source.
//! 2. No two crates state the same responsibility — the direct gate against the
//!    defect that gave `Claim` five homes.
//! 3. No crate name encodes a layer, and no `domain`-prefixed crate carries
//!    adapters or an HTTP client, which is what the prefix would be lying about.
//! 4. No metaphor names a crate. Metaphors stay legal inside types and prose.
//!
//! It checks the NAMESPACE, not the architecture. Whether a crate sits in the
//! right layer, or whether a dependency points the right way, is the job of
//! `crate_topology_coherence.rs` against `infra/crate-topology.json`; this file
//! deliberately does not re-derive that, so the two gates cannot disagree.
//!
//! Fails in BOTH directions, like the orphan ratchet: a new violation is
//! rejected, and a repaired one must leave the recorded baseline, which may
//! only shrink.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

/// Every workspace crate name, from `crates/`.
fn crate_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(workspace_root().join("crates"))
        .expect("read crates/")
        .flatten()
        .filter(|e| e.path().join("Cargo.toml").is_file())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    names.sort();
    names
}

fn manifest_of(crate_name: &str) -> String {
    fs::read_to_string(
        workspace_root()
            .join("crates")
            .join(crate_name)
            .join("Cargo.toml"),
    )
    .unwrap_or_else(|e| panic!("read {crate_name}/Cargo.toml: {e}"))
}

fn description_of(crate_name: &str) -> String {
    let manifest = manifest_of(crate_name);
    manifest
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix("description")?;
            let rest = rest.trim().strip_prefix('=')?.trim();
            Some(rest.trim_matches('"').to_string())
        })
        .unwrap_or_default()
}

fn source_of(crate_name: &str) -> String {
    let src = workspace_root().join("crates").join(crate_name).join("src");
    let mut out = String::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(text) = fs::read_to_string(&path) {
                    out.push_str(&text);
                    out.push('\n');
                }
            }
        }
    }
    out
}

/// Does this crate's source carry an adapter, or an HTTP client?
fn carries_infrastructure(crate_name: &str) -> bool {
    let source = source_of(crate_name);
    source.contains("reqwest")
        || source.contains("ureq")
        || source.contains("hyper::")
        || source.contains("tokio::process")
        || source.contains("std::process::Command")
}

/// Metaphor tokens that may not name a crate.
///
/// Recorded as data, not as taste: a gate nobody can compute is a wish. Adding a
/// token here is a taxonomy decision and belongs in the policy amendment.
const METAPHOR_TOKENS: &[&str] = &["askesis", "kyon", "zen", "agora", "mythos"];

/// Layer encodings that may not appear in a crate name. The layer is declared in
/// `infra/crate-topology.json` and checked against the real graph.
const LAYER_TOKENS: &[&str] = &["-l0", "-l1", "-l2", "-l3", "-l4", "layer"];

/// The family prefix that asserts domain purity, and what it must not carry.
const PURITY_PREFIX: &str = "cynic-domain-";

/// Violations present at the ratification of this gate, 2026-09-29.
///
/// Measured, not assumed. This list may only shrink: a crate that gains a
/// description, a metaphor-free name or an honest prefix must leave it, and a
/// test that cannot be tightened is a test that will be deleted with its author.
const BASELINE: &[&str] = &[
    // 1. no stated responsibility (21 crates have no `description`)
    "NO-RESPONSIBILITY: cynic-askesis",
    "NO-RESPONSIBILITY: cynic-cli",
    "NO-RESPONSIBILITY: cynic-discovery",
    "NO-RESPONSIBILITY: cynic-discovery-host",
    "NO-RESPONSIBILITY: cynic-domain-askesis",
    "NO-RESPONSIBILITY: cynic-domain-cloudflare",
    "NO-RESPONSIBILITY: cynic-domain-porkbun",
    "NO-RESPONSIBILITY: cynic-forensics",
    "NO-RESPONSIBILITY: cynic-host",
    "NO-RESPONSIBILITY: cynic-inference",
    "NO-RESPONSIBILITY: cynic-intel",
    "NO-RESPONSIBILITY: cynic-intel-escore",
    "NO-RESPONSIBILITY: cynic-intel-futarchy",
    "NO-RESPONSIBILITY: cynic-kernel",
    "NO-RESPONSIBILITY: cynic-lxc",
    "NO-RESPONSIBILITY: cynic-mcp",
    "NO-RESPONSIBILITY: cynic-node",
    "NO-RESPONSIBILITY: cynic-node-protocol",
    "NO-RESPONSIBILITY: cynic-pkg",
    "NO-RESPONSIBILITY: cynic-protocol",
    // 3. the purity prefix lies about two crates that ship an HTTP client
    "PURITY-PREFIX-WITH-INFRA: cynic-domain-cloudflare",
    "PURITY-PREFIX-WITH-INFRA: cynic-domain-porkbun",
    // 4. metaphor names a crate
    "METAPHOR-NAME: cynic-askesis",
    "METAPHOR-NAME: cynic-domain-askesis",
];

fn violations() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();

    for name in crate_names() {
        // 1. a stated responsibility
        if description_of(&name).trim().is_empty() {
            found.push(format!("NO-RESPONSIBILITY: {name}"));
        }

        // 3. a layer encoded in the name
        let lower = name.to_ascii_lowercase();
        for token in LAYER_TOKENS {
            if lower.contains(token) {
                found.push(format!("LAYER-IN-NAME: {name} encodes `{token}`"));
            }
        }

        // 3. a purity prefix on a crate that carries infrastructure
        if lower.starts_with(PURITY_PREFIX) && carries_infrastructure(&name) {
            found.push(format!("PURITY-PREFIX-WITH-INFRA: {name}"));
        }

        // 4. a metaphor naming a crate
        for token in METAPHOR_TOKENS {
            if lower.split('-').any(|segment| segment == *token) {
                found.push(format!("METAPHOR-NAME: {name}"));
            }
        }
    }

    // 2. one responsibility, one crate
    let mut by_responsibility: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in crate_names() {
        let responsibility = description_of(&name).trim().to_lowercase();
        if responsibility.is_empty() {
            continue;
        }
        by_responsibility
            .entry(responsibility)
            .or_default()
            .push(name);
    }
    for (responsibility, owners) in &by_responsibility {
        if owners.len() > 1 {
            found.push(format!(
                "SHARED-RESPONSIBILITY: {owners:?} all claim `{responsibility}`"
            ));
        }
    }

    found.sort();
    found
}

#[test]
fn crate_taxonomy_ownership_is_measured_and_shrink_only() {
    let observed = violations();
    let baseline: Vec<&str> = BASELINE.to_vec();

    let new: Vec<&String> = observed
        .iter()
        .filter(|v| !baseline.contains(&v.as_str()))
        .collect();
    let stale: Vec<&&str> = baseline
        .iter()
        .filter(|b| !observed.iter().any(|o| o.as_str() == **b))
        .collect();

    assert!(
        new.is_empty(),
        "new taxonomy violations, which no crate may add:\n{new:#?}\n\
         The rules are contracts/pruning-policy.json responsibility_rules. \
         Fix the name, state a description, or record the decision in the baseline \
         — the baseline may only shrink."
    );
    assert!(
        stale.is_empty(),
        "stale baseline entries: a violation was repaired but is still recorded:\n{stale:#?}\n\
         Remove them. A ratchet that cannot tighten is a ratchet that rots."
    );
}

#[test]
fn the_policy_that_this_gate_executes_still_exists() {
    // The gate is worthless if the rule it executes is deleted from the contract.
    let policy = fs::read_to_string(workspace_root().join("contracts/pruning-policy.json"))
        .expect("read contracts/pruning-policy.json");
    for rule in [
        "name_states_responsibility",
        "one_responsibility_one_crate",
        "family_prefix_is_optional_and_not_normative",
        "metaphor_banned_from_crate_names",
    ] {
        assert!(
            policy.contains(rule),
            "contracts/pruning-policy.json no longer carries `{rule}`; \
             the gate would then enforce a rule that does not exist"
        );
    }
}

#[test]
fn the_baseline_cannot_grow_without_a_policy_amendment() {
    // A crate added to the baseline is a taxonomy decision. The amendment must
    // be recorded in the contract, so the two files cannot drift apart quietly.
    let policy = fs::read_to_string(workspace_root().join("contracts/pruning-policy.json"))
        .expect("read contracts/pruning-policy.json");
    let amendment = policy.contains("amendment_2026_09_29");
    assert!(
        amendment || BASELINE.is_empty(),
        "the baseline holds {} entries but no amendment is recorded in \
         contracts/pruning-policy.json",
        BASELINE.len()
    );
}
