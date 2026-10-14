#![allow(clippy::expect_used)]

//! Constitutional repository tree governance test: Fail-closed admission gate.
//!
//! Enforces `contracts/repository-governance.json`:
//! - Deny-by-default for all untracked / undeclared root directories and files.
//! - Strict whitelist for root directories and root files.
//! - Strict blacklist for forbidden roots (scripts, specs, evidence, references, etc.).
//! - Strict blacklist for file types (binaries, office files, datasets, agent claims, secrets).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate is two levels below the workspace root")
        .to_path_buf()
}

fn governance_contract() -> Value {
    let path = workspace_root().join("contracts/repository-governance.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("contracts/repository-governance.json is valid JSON")
}

fn tracked_paths() -> BTreeSet<String> {
    let out = Command::new("git")
        .args(["ls-files"])
        .current_dir(workspace_root())
        .output()
        .expect("run git ls-files");
    assert!(out.status.success(), "git ls-files must succeed");
    String::from_utf8(out.stdout)
        .expect("utf-8 git output")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn repository_governance_contract_is_valid() {
    let contract = governance_contract();
    assert_eq!(
        contract["schema"].as_str(),
        Some("cynic.repository_governance.v1"),
        "schema must match cynic.repository_governance.v1"
    );
    assert_eq!(
        contract["policy"].as_str(),
        Some("deny_by_default"),
        "policy must be deny_by_default"
    );
}

#[test]
fn root_admission_whitelist_holds() {
    let contract = governance_contract();
    let allowed_dirs: BTreeSet<String> = contract["root_admission"]["allowed_directories"]
        .as_array()
        .expect("allowed_directories is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();

    let allowed_files: BTreeSet<String> = contract["root_admission"]["allowed_files"]
        .as_array()
        .expect("allowed_files is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();

    let mut violations = Vec::new();
    for path in tracked_paths() {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() == 1 {
            // Root file
            let file = parts[0];
            if !allowed_files.contains(file) {
                violations.push(format!(
                    "Root file '{file}' is not in allowed_files whitelist"
                ));
            }
        } else {
            // File in a directory
            let root_dir = parts[0];
            if !allowed_dirs.contains(root_dir) {
                violations.push(format!(
                    "Path '{path}' is in root directory '{root_dir}', which is not in allowed_directories whitelist"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Root admission violations detected (deny-by-default):\n{}",
        violations.join("\n")
    );
}

#[test]
fn forbidden_root_directories_are_absent() {
    let contract = governance_contract();
    let forbidden_roots: BTreeSet<String> = contract["forbidden_roots"]
        .as_array()
        .expect("forbidden_roots is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();

    let mut violations = Vec::new();
    for path in tracked_paths() {
        let root_segment = path.split('/').next().unwrap_or("");
        if forbidden_roots.contains(root_segment) {
            violations.push(format!(
                "Path '{path}' belongs to forbidden root directory '{root_segment}'"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Forbidden root directories present in repository:\n{}",
        violations.join("\n")
    );
}

#[test]
fn blacklisted_file_patterns_are_strictly_absent() {
    let contract = governance_contract();
    let patterns: Vec<String> = contract["blacklisted_file_patterns"]
        .as_array()
        .expect("blacklisted_file_patterns is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();

    let mut violations = Vec::new();
    for path in tracked_paths() {
        let file_name = path.split('/').last().unwrap_or("");
        for pat in &patterns {
            let matches = if pat.starts_with("*.") {
                let ext = &pat[1..]; // e.g. ".pptx"
                file_name.ends_with(ext)
            } else if pat.ends_with("*") {
                let prefix = &pat[..pat.len() - 1];
                file_name.starts_with(prefix)
            } else {
                file_name == pat || path == *pat
            };

            if matches {
                violations.push(format!(
                    "Tracked file '{path}' matches blacklisted pattern '{pat}'"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Blacklisted files tracked in git:\n{}",
        violations.join("\n")
    );
}

#[test]
fn state_leak_blacklist_is_strictly_enforced() {
    let contract = governance_contract();
    let leak_patterns: Vec<String> = contract["state_leak_blacklist"]
        .as_array()
        .expect("state_leak_blacklist is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();

    let mut violations = Vec::new();
    for path in tracked_paths() {
        // Skip declared surface descriptors in surface-registry
        if path.starts_with("infra/surface-registry/") {
            continue;
        }

        let file_name = path.split('/').last().unwrap_or("");
        for pat in &leak_patterns {
            let matches = if pat.starts_with("*") && pat.ends_with(".json") {
                let suffix = &pat[1..];
                file_name.ends_with(suffix)
            } else if pat.starts_with("*-") {
                let suffix = &pat[1..];
                file_name.ends_with(suffix)
            } else {
                file_name == pat || path == *pat
            };

            if matches {
                violations.push(format!(
                    "Dynamic state leak detected: '{path}' matches forbidden pattern '{pat}'"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Dynamic state leaks tracked in git (must move to data_system/):\n{}",
        violations.join("\n")
    );
}
