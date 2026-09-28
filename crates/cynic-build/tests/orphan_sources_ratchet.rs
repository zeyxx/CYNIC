//! Orphan-source ratchet for `cynic-build`.
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
    "build/bootstrap_recovery.rs",];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-build", QUARANTINED);
}
