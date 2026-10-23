//! Session lifecycle governance test: fail-closed admission gate.
//!
//! Enforces `contracts/session-lifecycle.json`:
//! - Six bounded states with `closed` as the single terminal state.
//! - Every transition endpoint is a declared state (no prose-only states).
//! - Exactly the five measured counters, each with kind, producer, and fields.
//! - GC collectability requires the two-fact rule, never a single heuristic.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate is two levels below the workspace root")
        .to_path_buf()
}

fn session_contract() -> Value {
    let path = workspace_root().join("contracts/session-lifecycle.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("contracts/session-lifecycle.json is valid JSON")
}

#[test]
fn session_lifecycle_schema_and_terminal_hold() {
    let contract = session_contract();
    assert_eq!(
        contract["contract_id"].as_str(),
        Some("cynic.session_lifecycle.v1"),
        "contract_id must match cynic.session_lifecycle.v1"
    );
    let states: BTreeSet<String> = contract["session_state_machine"]["states"]
        .as_array()
        .expect("states is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();
    assert_eq!(
        states.len(),
        6,
        "exactly six session states, got {states:?}"
    );
    for expected in [
        "opened",
        "claimed",
        "working",
        "proving",
        "releasing",
        "closed",
    ] {
        assert!(states.contains(expected), "missing state {expected}");
    }
    assert_eq!(
        contract["session_state_machine"]["terminal"].as_str(),
        Some("closed"),
        "terminal must be closed"
    );
}

#[test]
fn session_transitions_are_closed_over_states() {
    let contract = session_contract();
    let states: BTreeSet<String> = contract["session_state_machine"]["states"]
        .as_array()
        .expect("states is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();
    let transitions = contract["session_state_machine"]["transitions"]
        .as_array()
        .expect("transitions is array");
    assert!(!transitions.is_empty(), "transitions must not be empty");
    let mut offenders = Vec::new();
    for t in transitions {
        for end in ["from", "to"] {
            let name = t[end].as_str().unwrap_or("?");
            if !states.contains(name) {
                offenders.push(format!("{end}={name}"));
            }
        }
        let requires = t["requires"].as_array();
        if requires.map_or(true, |r| r.is_empty()) {
            offenders.push("transition without requires".to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "transition endpoints must be declared states with requires: {}",
        offenders.join(", ")
    );
}

#[test]
fn session_counters_are_exactly_five_with_provenance() {
    let contract = session_contract();
    let counters = contract["counters"].as_array().expect("counters is array");
    let mut kinds = BTreeSet::new();
    let mut offenders = Vec::new();
    for c in counters {
        let kind = c["kind"].as_str().unwrap_or("?");
        kinds.insert(kind.to_string());
        for key in ["kind", "producer", "consumer", "fields", "counts"] {
            if c.get(key).is_none() {
                offenders.push(format!("{kind} missing {key}"));
            }
        }
        if c["fields"].as_array().map_or(true, |f| f.is_empty()) {
            offenders.push(format!("{kind} has empty fields"));
        }
    }
    for expected in [
        "toll_paid",
        "gc_decision",
        "base_drift",
        "head_collision",
        "claim_expiry",
    ] {
        assert!(kinds.contains(expected), "missing counter {expected}");
    }
    assert_eq!(kinds.len(), 5, "exactly five counters, got {kinds:?}");
    assert!(
        offenders.is_empty(),
        "counters need full provenance: {}",
        offenders.join(", ")
    );
}

#[test]
fn session_gc_rule_names_no_single_heuristic() {
    let contract = session_contract();
    let never: Vec<String> = contract["gc_rule"]["never"]
        .as_array()
        .expect("gc_rule.never is array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();
    for banned in ["clean alone", "prunable alone", "age alone", "size alone"] {
        assert!(
            never.iter().any(|n| n == banned),
            "gc_rule.never must ban {banned}"
        );
    }
    assert!(
        contract["gc_rule"]["rule"]
            .as_str()
            .unwrap_or("")
            .contains("released"),
        "gc rule must require claim release"
    );
    assert!(
        contract["gc_rule"]["rule"]
            .as_str()
            .unwrap_or("")
            .contains("reachable"),
        "gc rule must require head reachability"
    );
}
