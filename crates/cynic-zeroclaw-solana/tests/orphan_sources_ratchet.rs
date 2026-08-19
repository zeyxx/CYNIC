//! Orphan-source ratchet for `cynic-zeroclaw-solana`.
//!
//! Shares one detector with every other crate (`tests/support/orphan_sources.rs`)
//! so reachability rules are defined once. See that file for why `mod`,
//! `#[path]`, `include!`, and `Cargo.toml` targets must all be followed.
//!
//! This crate has NO orphans: its only candidate, `src/bin/demo.rs`, is a
//! declared `[[bin]]` target. A probe that removed it broke the build with
//! `can't find bin zeroclaw-solana-demo`, which is how the detector's original
//! blind spot was found — manifest targets are compilation roots too.

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;

/// No quarantined sources: everything here is reachable.
const QUARANTINED: &[&str] = &[];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-zeroclaw-solana", QUARANTINED);
}
