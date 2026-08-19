//! Orphan-source ratchet for `cynic-kernel`.
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
    "api/inference_client.rs",
    "api/mcp/tests.rs",
    "api/rest/temporal.rs",
    "backends/auto_embed.rs",
    "domain/resource_guard.rs",
    "infra/coordination_authority.rs",
    "infra/coordination_journal.rs",
    "infra/judge/mod.rs",
    "infra/judge/parser.rs",
    "infra/judge/prompt.rs",
    "infra/local_spool.rs",
    "infra/runtime_takeover.rs",
    "infra/tasks/runtime_loops/events.rs",
    "infra/tasks/runtime_loops/healing.rs",
    "infra/tasks/runtime_loops/heartbeats.rs",
    "infra/tasks/runtime_loops/metrics.rs",
    "infra/tasks/runtime_loops/perf.rs",
    "infra/tasks/runtime_loops/probes.rs",
    "infra/tasks/takeover.rs",
    "organ/backend_handle.rs",
    "organ/inference_organ.rs",
    "organ/tests.rs",
    "temporal_clock.rs",
];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-kernel", QUARANTINED);
}
