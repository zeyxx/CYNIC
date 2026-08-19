//! Orphan-source ratchet for `cynic-protocol`.
//!
//! Shares one detector with every other crate (`tests/support/orphan_sources.rs`)
//! so reachability rules are defined once. See that file for why `mod`,
//! `#[path]`, `include!`, and `Cargo.toml` targets must all be followed.
//!
//! The list below is measured, not assumed: every entry was confirmed
//! unreachable by the shared detector, and the detector itself was validated by
//! probe — removing a file and rebuilding, so a live file cannot be recorded as
//! dead. The test fails in both directions: a new orphan is rejected, and a
//! repaired file must be removed from the list.

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;

/// Sources currently unreachable from this crate's roots.
///
/// Quarantined as explicit data with the debt visible, NOT tolerated: nothing
/// compiles these files, so any contract they claim to enforce is inert. The
/// list may only shrink.
const QUARANTINED: &[&str] = &[
    "agent_execution_provenance.rs",
    "audit.rs",
    "build/bootstrap_recovery.rs",
    "coordination_authority.rs",
    "coordination_fault_harness.rs",
    "coordination_journal.rs",
    "coordination_scaling_harness.rs",
    "detection.rs",
    "eval_hygiene.rs",
    "fleet_inference/mod.rs",
    "fleet_inference/policy.rs",
    "fleet_inference/types.rs",
    "hardening.rs",
    "runtime/degradation.rs",
    "runtime_placement.rs",
    "source_change_submission.rs",
];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-protocol", QUARANTINED);
}
