//! Artefact contract guard.
//!
//! WHY. The workspace produced 13 binaries while `infra/runtime-basket.json`
//! declared 5, and nothing compared the two. Three sets existed -- built,
//! declared, installed -- and none was checked against another. The gap was
//! invisible because `autobins` was on: a file dropped in `src/bin/` became a
//! deployment artefact with no declaration anywhere.
//!
//! `autobins = false` is now set in every crate, so the manifests are the
//! complete and authoritative list of what the compiler produces. This guard
//! compares that list against the contract.
//!
//! HOW. Pure file reading: every `crates/*/Cargo.toml` `[[bin]]` name, against
//! `runtime-basket.json` (artifacts + tooling + the runtime seed executor). No
//! subprocess, so the guard is deterministic and runs anywhere.
//!
//! NEGATIVE BOUNDARY. It does not build, install, or classify anything. It
//! asserts that the declared set and the produced set are the same set, and
//! names the difference in both directions.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("contracts/foundation.json").is_file() && dir.join("Cargo.toml").is_file() {
            return dir;
        }
        if !dir.pop() {
            panic!(
                "workspace root not found above {}",
                env!("CARGO_MANIFEST_DIR")
            );
        }
    }
}

/// Binary names declared by `[[bin]]` sections across the workspace.
///
/// With `autobins = false` this is exhaustive: a binary absent here is not
/// built, and a binary built but absent here is impossible.
fn declared_binaries(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let crates = root.join("crates");
    let entries = fs::read_dir(&crates).expect("read crates/");
    for entry in entries.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let text = fs::read_to_string(&manifest).expect("read manifest");
        // A [[bin]] section's `name = "..."` line. `path = "..."` is skipped
        // because it does not start with `name`.
        let mut in_bin = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed == "[[bin]]" {
                in_bin = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_bin = false;
                continue;
            }
            if in_bin {
                if let Some(rest) = trimmed.strip_prefix("name = ") {
                    let name = rest.trim().trim_matches('"');
                    if !name.is_empty() {
                        found.insert(name.to_string());
                    }
                }
            }
        }
    }
    found
}

/// Binary names the contract accounts for: shipped artefacts, tooling, and the
/// runtime seed (which is deliberately not a basket member).
fn contracted_binaries(root: &Path) -> BTreeSet<String> {
    let path = root.join("infra/runtime-basket.json");
    let text = fs::read_to_string(&path).expect("read runtime-basket.json");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("parse runtime-basket.json");

    let mut found = BTreeSet::new();
    for key in ["artifacts", "tooling"] {
        let entries = doc
            .get(key)
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("runtime-basket.json must declare `{key}`"));
        for entry in entries {
            let name = entry
                .get("binary")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("a {key} entry has no `binary`"));
            found.insert(name.to_string());
        }
    }
    let seed = doc
        .get("runtime_seed")
        .and_then(|v| v.get("executor"))
        .and_then(|v| v.as_str())
        .expect("runtime_seed.executor");
    found.insert(seed.to_string());
    found
}

/// Every binary the compiler produces must be accounted for in the contract.
/// An undeclared binary is a deployment artefact nobody agreed to ship.
#[test]
fn every_declared_binary_is_accounted_for_in_the_contract() {
    let root = workspace_root();
    let declared = declared_binaries(&root);
    let contracted = contracted_binaries(&root);

    assert!(
        !declared.is_empty(),
        "no [[bin]] found: the guard would pass vacuously"
    );

    let undeclared: Vec<&String> = declared.difference(&contracted).collect();
    assert!(
        undeclared.is_empty(),
        "the workspace builds binaries absent from infra/runtime-basket.json:\n{undeclared:#?}\n\
         Add each one to `artifacts` (deployed) or `tooling` (never deployed)."
    );
}

/// And the contract must not promise a binary the workspace does not build.
/// A phantom entry makes every role bundle select an artefact that cannot exist.
#[test]
fn every_contracted_binary_is_actually_built() {
    let root = workspace_root();
    let declared = declared_binaries(&root);
    let contracted = contracted_binaries(&root);

    let phantom: Vec<&String> = contracted.difference(&declared).collect();
    assert!(
        phantom.is_empty(),
        "infra/runtime-basket.json declares binaries the workspace does not build:\n{phantom:#?}\n\
         A role bundle selecting one of these blocks. Remove the entry or restore the target."
    );
}

/// The audiences must partition the contracted set: each audience present, and
/// no entry without one. An unclassified artefact cannot be shipped or excluded
/// deliberately.
#[test]
fn every_shipped_artifact_carries_a_known_audience() {
    let root = workspace_root();
    let text = fs::read_to_string(root.join("infra/runtime-basket.json")).expect("read");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("parse");

    let known: BTreeSet<&str> = doc
        .get("audiences")
        .and_then(|v| v.as_object())
        .expect("audiences")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        known,
        BTreeSet::from(["operator", "service", "privileged", "tooling"]),
        "the audience set is fixed; adding one is a policy change"
    );

    for key in ["artifacts", "tooling"] {
        for entry in doc.get(key).and_then(|v| v.as_array()).expect(key) {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let audience = entry
                .get("audience")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("{id} has no audience"));
            assert!(
                known.contains(audience),
                "{id} declares unknown audience `{audience}`"
            );
        }
    }
}
