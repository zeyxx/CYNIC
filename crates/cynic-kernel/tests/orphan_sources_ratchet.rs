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
///
/// 2026-09-28 purge (opencode:purge-morte): 8 confirmed-dead orphans deleted
/// (582 LOC: api/rest/temporal.rs, temporal_clock.rs, 6x runtime_loops/*);
/// 5 stale entries removed (already gone from disk); local_spool.rs removed
/// (live: declared by infra/mod.rs, reachable). List emptied; residual
/// under-quarantined orphans elsewhere in the crate remain explicit unknown.
const QUARANTINED: &[&str] = &[];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-kernel", QUARANTINED);
}
