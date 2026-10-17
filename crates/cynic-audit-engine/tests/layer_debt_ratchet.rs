//! Ratchet on the tolerated layer debt: the ascending edges ARCH-004 reports.
//!
//! WHY THIS FILE EXISTS. `layer_rule` in `infra/crate-topology.json` counted the
//! ascending L1 -> L2 edges in prose — "12 ascending edges ... counted,
//! documented and unfixed" — and nothing checked that count. A number written in
//! a sentence is not a measurement: it cannot fail, so it cannot be paid. The
//! debt grew to 15 while the sentence still said 12, and the two in-flight
//! edges (`cynic-host-probes -> cynic-finance`, `cynic-network -> cynic-finance`)
//! were added by a domain landing in two substrate crates.
//!
//! WHAT IT DOES. Runs the real ARCH-004 rule over the real workspace and holds
//! three things at once:
//!
//! 1. the manifest's `layer_debt.edges` equals what the engine reports, exactly
//!    — so the SSOT cannot drift from reality in either direction;
//! 2. no edge outside `BASELINE_EDGES` may exist, so debt cannot be added
//!    silently;
//! 3. the observed count may not exceed `BASELINE_EDGES.len()`, so raising the
//!    manifest alone cannot silence this gate.
//!
//! Paying an edge removes it from both the manifest and the baseline below, in
//! the same commit, which is what makes the reduction visible in a diff.
//!
//! WHY THE COHERENCE GATE IS STILL RED. `crate_topology_coherence.rs` asserts
//! that the upward-edge set is EMPTY, and it is correct to. It stays red until
//! the last edge is paid; that is the intent, not an oversight. This file does
//! not relax it, it measures the distance to green.

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

fn ascending_edges() -> Vec<String> {
    // The RAW evaluation, not the verdict. `run_workspace_audit` applies the
    // debt ledger, which absorbs every violation the ledger declares — and all
    // fifteen ascending edges are declared. Reading the verdict therefore
    // returns an empty set, and comparing it against a fifteen-entry baseline
    // fails for a reason that has nothing to do with the layer model.
    let report = cynic_audit_engine::evaluate_workspace_rules(&workspace_root());
    assert!(
        report
            .evaluated_rules
            .iter()
            .any(|spec| spec.id == "ARCH-004"),
        "ARCH-004 missing from the default rule set, so this ratchet would measure nothing"
    );
    let mut edges: Vec<String> = report
        .violations
        .iter()
        .filter(|violation| violation.code == "ARCH-004-UPWARD-EDGE")
        // code + message, the same shape `crate_topology_coherence.rs` asserts
        // on, so the two gates can never disagree about what an edge is
        .map(|violation| format!("{} {}", violation.code, violation.message))
        .collect();
    edges.sort();
    edges
}

fn manifest_debt() -> (Vec<String>, usize) {
    let text = std::fs::read_to_string(workspace_root().join("infra/crate-topology.json"))
        .expect("read infra/crate-topology.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse crate-topology.json");
    let debt = value
        .get("layer_debt")
        .unwrap_or_else(|| panic!("infra/crate-topology.json has no `layer_debt` field"));
    let edges: Vec<String> = debt["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("layer_debt.edges is not an array"))
        .iter()
        .map(|e| {
            e.as_str()
                .unwrap_or_else(|| panic!("layer_debt.edges holds a non-string: {e}"))
                .to_string()
        })
        .collect();
    let count = debt["count"].as_u64().expect("layer_debt.count is not a number") as usize;
    (edges, count)
}

/// The tolerated debt, recorded when this ratchet was written.
///
/// Shrink-only, and deliberately duplicated from the manifest: if the only copy
/// lived in the SSOT, a single edit would raise the ceiling and the gate would
/// agree with itself. Two copies in two files means paying debt is a visible
/// act and adding debt is a diff in both.
const BASELINE_EDGES: &[&str] = &[
    "ARCH-004-UPWARD-EDGE cynic-discovery-host (L1) depends on cynic-build (L2)",
    "ARCH-004-UPWARD-EDGE cynic-discovery-host (L1) depends on cynic-discovery (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-build (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-eval (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-finance (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-inference (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-intel (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-node-protocol (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-storage (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-vcs (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host (L1) depends on cynic-vcs-forgejo (L2)",
    "ARCH-004-UPWARD-EDGE cynic-host-probes (L1) depends on cynic-finance (L2)",
    "ARCH-004-UPWARD-EDGE cynic-lifecycle (L1) depends on cynic-build (L2)",
    "ARCH-004-UPWARD-EDGE cynic-lifecycle (L1) depends on cynic-storage (L2)",
    "ARCH-004-UPWARD-EDGE cynic-network (L1) depends on cynic-finance (L2)",
];

#[test]
fn the_manifest_debt_and_the_engine_agree_exactly() {
    let (declared, count) = manifest_debt();
    assert_eq!(
        count,
        declared.len(),
        "layer_debt.count ({count}) does not match the number of edges ({})",
        declared.len()
    );

    let mut sorted = declared.clone();
    sorted.sort();
    assert_eq!(
        sorted, ascending_edges(),
        "the recorded layer debt and what ARCH-004 reports have diverged.\n\
         recorded: {sorted:#?}\n\
         observed: {:#?}",
        ascending_edges()
    );
}

#[test]
fn the_tolerated_debt_may_only_shrink() {
    let observed = ascending_edges();
    assert!(
        observed.len() <= BASELINE_EDGES.len(),
        "layer debt grew from {} to {} edges. An ascending edge is a new architectural \
         violation: it was either a bad decision or an unrecorded debt. Either way it \
         needs a written reason and a tranche, not a silent arrival.\n\
         new: {observed:#?}",
        BASELINE_EDGES.len(),
        observed.len()
    );
    for edge in &observed {
        assert!(
            BASELINE_EDGES.contains(&edge.as_str()),
            "an ascending edge is not in the recorded baseline: `{edge}`. \
             Either pay it or record it with a reason — do not both."
        );
    }
}

#[test]
fn the_baseline_has_no_paid_entries() {
    // A paid edge must leave the baseline too, otherwise the ceiling stops
    // describing the debt and the count comparison above becomes meaningless.
    let observed = ascending_edges();
    let stale: Vec<&&str> = BASELINE_EDGES
        .iter()
        .filter(|b| !observed.iter().any(|o| o.as_str() == **b))
        .collect();
    assert!(
        stale.is_empty(),
        "these edges are no longer ascending but are still recorded as debt; \
         remove them from BASELINE_EDGES and from layer_debt.edges:\n{stale:#?}"
    );
}
