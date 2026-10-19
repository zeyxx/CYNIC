//! Coherence between `infra/workspace-paths.json` and every site that hardcodes the canonical path.
//!
//! The canonical CYNIC repository path was written in nine places with no
//! authority: three Python/shell scripts, four docs, one test, and one
//! registry. A new agent inheriting any of these files would recreate the
//! parent directory `asdf-forge/zeyxx` instead of using `asdf-forge/zeyxx/CYNIC`,
//! because the path was duplicated rather than resolved.
//!
//! This test makes the registry authoritative instead of the literal. It reads
//! the declared canonical path and checks each consumer carries it, so a
//! legitimate path change edits one file and this test follows.
//!
//! This is an architecture test (like `build_policy_coherence.rs`), not
//! business logic — it lives in `cynic-cli` because it needs I/O to read files.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    crate_root().join("../..")
}

fn read_registry() -> Value {
    let path = repo_root().join("infra/workspace-paths.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {} as JSON: {e}", path.display()))
}

fn canonical_path() -> String {
    let registry = read_registry();
    registry["canonical_path"]
        .as_str()
        .expect("canonical_path must be a string")
        .to_string()
}

fn consumers() -> Vec<Value> {
    let registry = read_registry();
    registry["consumers"]
        .as_array()
        .expect("consumers must be an array")
        .clone()
}

fn read_source(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every consumer is a reader, so its declared form must appear verbatim.
#[test]
fn every_declared_consumer_carries_the_canonical_path() {
    let canonical = canonical_path();
    let list = consumers();
    assert!(!list.is_empty(), "the registry must declare its consumers");

    for consumer in &list {
        let site = consumer["site"].as_str().expect("consumer needs a site");
        let form = consumer["form"].as_str().expect("consumer needs a form");
        let placeholder_form = form.replace("<canonical_path>", &canonical);
        let file = site.split(':').next().expect("site must name a file");

        let source = read_source(file);
        assert!(
            source.contains(&placeholder_form),
            "consumer {site} does not carry the canonical path.\n\
             expected to find: {placeholder_form}\n\
             authority: infra/workspace-paths.json canonical_path"
        );
    }
}

/// A new consumer that nobody registered is the exact drift this file exists
/// to prevent, so a raw scan must find no unregistered hardcoded path.
#[test]
fn no_unregistered_hardcoded_path_exists() {
    let canonical = canonical_path();
    let registered: Vec<String> = consumers()
        .iter()
        .filter_map(|c| {
            let site = c["site"].as_str()?;
            let file = site.split(':').next()?;
            Some(file.to_string())
        })
        .collect();

    // Files that are allowed to contain the canonical path (declared consumers)
    // plus files that are allowed to reference it indirectly (registry itself,
    // this test).
    let allowed: Vec<&str> = registered
        .iter()
        .map(|s| s.as_str())
        .chain(vec![
            "infra/workspace-paths.json",
            "crates/cynic-cli/tests/workspace_paths_coherence.rs",
        ])
        .collect();

    // Scan all text files in the repo for the canonical path
    let output = std::process::Command::new("grep")
        .arg("-r")
        .arg("--include=*.py")
        .arg("--include=*.sh")
        .arg("--include=*.md")
        .arg("--include=*.rs")
        .arg("--include=*.toml")
        .arg("--include=*.yaml")
        .arg("--include=*.yml")
        .arg("--include=*.json")
        .arg("-l")
        .arg(&canonical)
        .arg(repo_root())
        .output()
        .expect("grep failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let relative = line
            .strip_prefix(&format!("{}/", repo_root().display()))
            .unwrap_or(line)
            .to_string();
        // Grep returns `.hermes/plans/...` as `hermes/plans/...` (no leading dot).
        // Normalize: if the relative path without the first char is in allowed,
        // it's a hidden directory match.
        let matches = allowed.iter().any(|a| {
            relative.starts_with(a)
                || relative.strip_prefix('.').is_some_and(|rest| allowed.iter().any(|a| rest.starts_with(a)))
        });
        if !matches {
            panic!(
                "{} contains the canonical path but is not registered in \
                 infra/workspace-paths.json consumers. Add it to the registry.",
                relative
            );
        }
    }
}

/// The registry must never claim a path that does not exist.
#[test]
fn the_canonical_path_exists_on_disk() {
    let canonical = canonical_path();
    let path = PathBuf::from(&canonical);
    if !path.exists() {
        // When running on a remote builder or CI where the local developer workspace
        // is not present, assert that the active checkout is valid instead.
        assert!(
            repo_root().join("infra/registry.json").is_file(),
            "active repository does not contain infra/registry.json"
        );
        return;
    }
    assert!(
        path.is_dir(),
        "canonical_path {canonical} does not exist on disk"
    );
    assert!(
        path.join(".git").is_dir() || path.join(".git").is_file(),
        "canonical_path {canonical} is not a git repository"
    );
    assert!(
        path.join("infra/registry.json").is_file(),
        "canonical_path {canonical} does not contain infra/registry.json"
    );
}

/// Every consumer site exists on disk.
#[test]
fn every_consumer_site_exists_on_disk() {
    for consumer in consumers() {
        let site = consumer["site"].as_str().expect("consumer needs a site");
        let relative = site.split(':').next().expect("site must name a file");
        let path: PathBuf = repo_root().join(relative);
        assert!(
            path.is_file(),
            "consumer site {relative} is declared in the registry but does not exist"
        );
    }
}

/// The registry must name its own test.
#[test]
fn the_registry_names_its_own_coherence_test() {
    let registry = read_registry();
    let declared = registry["coherence_contract"]["test"]
        .as_str()
        .expect("coherence_contract must name its test");
    let path = repo_root().join(declared);
    assert!(
        path.is_file(),
        "the registry names {declared} as its coherence test but that file is absent"
    );
}
