// The shared detector is designed to be included by binaries that use different
// halves of it. `dead_code = "deny"` at workspace level otherwise fails the build
// for the unused half, which means this ratchet would not execute at all.
#![allow(dead_code)]

//! Orphan-source ratchet for `cynic-node`.
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
///
/// 2026-09-28: 30 entries -> 2, after the causal audit recorded in
/// `infra/WORKSPACE_ORPHAN_DECISIONS.md` §6. Twenty-eight were deleted because
/// they were second paths to capabilities a live crate already owns
/// (takeover activation -> cynic-node-protocol, mcp observation -> cynic-host,
/// local agent session -> cynic-kernel, source change -> cynic-workload,
/// runtime release -> cynic-build, federated bootstrap -> cynic-node-protocol,
/// runtime proof -> cynic-host + cynic-protocol) or because nothing demanded
/// them at all. The two below are the only pair a LIVE surface names as
/// missing: `runtime_federation.rs` and `websocket/domain_dispatch.rs` both
/// report the blocker `runtime_federation_workload_signer_unavailable`.
///
/// That pair is kept, not deleted, and the reason is the rule §6 states: what
/// has value is a capability a live surface demands. Deleting the only
/// candidate implementation of a gap the code reports by name would close no
/// gap and would erase the lead. It stays quarantined, visible and shrink-only:
/// wire it or delete it, do not leave it invisible.
const QUARANTINED: &[&str] = &[
    "adapters/local_workload_signer.rs",
    "ports/workload_signer_port.rs",
];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-node", QUARANTINED);
}
