// The shared detector is designed to be included by binaries that use different
// halves of it. `dead_code = "deny"` at workspace level otherwise fails the build
// for the unused half, which means this ratchet would not execute at all.
#![allow(dead_code)]

//! Orphan-source ratchet for `cynic-askesis`.
//!
//! Shares one detector with every other crate (`tests/support/orphan_sources.rs`)
//! so the reachability rules are defined once. See that file for why `mod`,
//! `#[path]`, `include!`, `Cargo.toml` targets and `src/bin/*.rs` must all be followed.
//!
//! Generated from the detector's own output, not from a separate resolver: a hand
//! count and the instrument disagree, and the instrument is the authority.
//!
//! The list is measured, not assumed, and it may only shrink. Nothing compiles
//! these files, so any contract they appear to enforce is inert.

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;

/// Sources currently unreachable from this crate's compilation roots.
const QUARANTINED: &[&str] = &[
    "agent_loop.rs",
    "audit/kernel_rest.rs",
    "audit/mod.rs",
    "error.rs",
    "eval/dataset.rs",
    "eval/mod.rs",
    "eval/openai.rs",
    "eval/oracle.rs",
    "eval/policy.rs",
    "eval/port.rs",
    "eval/runner.rs",
    "eval/types.rs",
    "experiment/mod.rs",
    "experiment/plan.rs",
    "experiment/port.rs",
    "experiment/runner.rs",
    "experiment/simulator.rs",
    "experiment/types.rs",
    "memory.rs",];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-askesis", QUARANTINED);
}
