//! The declared crate topology must equal the workspace that is actually built.
//!
//! WHY: the layer taxonomy lived as prose in AGENTS.md and tables in
//! infra/CANONICAL_CRATE_MAP.md, and the only boundary rule in the engine grepped
//! four strings inside cynic-protocol without ever reading the graph. An upward
//! edge, an undeclared crate, a cycle, or a crate nothing depends on were all
//! invisible while the tree stayed green.
//!
//! HOW: run the real rule over the real workspace and assert the checks that
//! matter are actually firing. A test asserting "the report has a schema" would
//! pass with every check disabled, which is how a broken audit stays green.

#![allow(clippy::expect_used)]

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

fn topology_violations() -> Vec<String> {
    let report = cynic_host::audit_engine::run_workspace_audit(&workspace_root());
    assert!(
        report
            .evaluated_rules
            .iter()
            .any(|spec| spec.id == "ARCH-004"),
        "ARCH-004 missing from the default rule set"
    );
    report
        .violations
        .iter()
        .filter(|violation| violation.code.starts_with("ARCH-004"))
        .map(|violation| format!("{} {}", violation.code, violation.message))
        .collect()
}

#[test]
fn the_topology_rule_is_registered_and_running() {
    // The rule must be reachable from the default set, or `cynic audit crate` and
    // MCP never evaluate it.
    let report = cynic_host::audit_engine::run_workspace_audit(&workspace_root());
    let ids: Vec<&str> = report
        .evaluated_rules
        .iter()
        .map(|spec| spec.id.as_str())
        .collect();
    assert!(ids.contains(&"ARCH-004"), "evaluated rules: {ids:?}");
}

#[test]
fn every_workspace_crate_is_declared_in_the_topology() {
    let reported = topology_violations();
    let undeclared: Vec<&String> = reported
        .iter()
        .filter(|line| line.starts_with("ARCH-004-UNDECLARED-CRATE"))
        .collect();
    let phantom: Vec<&String> = reported
        .iter()
        .filter(|line| line.starts_with("ARCH-004-PHANTOM-CRATE"))
        .collect();
    assert!(
        undeclared.is_empty() && phantom.is_empty(),
        "the topology SSOT and the build graph disagree:\n{undeclared:#?}\n{phantom:#?}"
    );
}

#[test]
fn the_declared_edges_are_the_real_edges() {
    let reported = topology_violations();
    let missing: Vec<&String> = reported
        .iter()
        .filter(|line| line.starts_with("ARCH-004-DECLARED-EDGE-ABSENT"))
        .collect();
    let extra: Vec<&String> = reported
        .iter()
        .filter(|line| line.starts_with("ARCH-004-UNDECLARED-EDGE"))
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "edge drift between the SSOT and the manifests:\ndeclared but absent: {missing:#?}\npresent but undeclared: {extra:#?}"
    );
}

#[test]
fn the_graph_is_acyclic() {
    let reported = topology_violations();
    let cycles: Vec<&String> = reported
        .iter()
        .filter(|line| line.starts_with("ARCH-004-CYCLE"))
        .collect();
    assert!(cycles.is_empty(), "cycle reported: {cycles:#?}");
}

/// The ten unjustified crates are DECLARED, so the rule is green today, and a new
/// unjustified crate cannot appear without the gate going red. That is the point
/// of `pending_disposition`: the debt is named and counted, not tolerated
/// silently and not hidden by listing more crates.
#[test]
fn unjustified_crates_are_declared_and_no_undisclosed_one_can_appear() {
    let reported = topology_violations();
    let undisclosed: Vec<&String> = reported
        .iter()
        .filter(|line| line.starts_with("ARCH-004-UNJUSTIFIED-CRATE"))
        .collect();
    assert!(
        undisclosed.is_empty(),
        "a crate with no dependents, no binary target and no disposition slipped in: {undisclosed:#?}"
    );

    // The declared list is the shrink-only ledger. Assert its size so removing a
    // crate without retiring its entry is caught.
    let ssot = std::fs::read_to_string(workspace_root().join("infra/crate-topology.json"))
        .expect("topology ssot");
    let document: serde_json::Value = serde_json::from_str(&ssot).expect("valid json");
    let declared = document
        .get("pending_disposition")
        .and_then(|value| value.as_object())
        .map(|entries| entries.len())
        .unwrap_or_default();
    assert_eq!(
        declared, 10,
        "pending_disposition changed size. A crate was fused or deleted: remove its entry. \
         A crate was added without justification: that is the failure this gate exists to stop."
    );
}
