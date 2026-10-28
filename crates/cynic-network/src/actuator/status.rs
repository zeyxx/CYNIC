//! Pure helpers for network actuator bundle completeness checks.
//!
//! Offloaded from `cynic-node` per Foundation §481 (SoC). I/O-heavy bundle
//! creation remains in `cynic-node`; only pure predicates live here.

pub fn bundle_complete(required_files: &[&str], present_files: &[&str]) -> bool {
    required_files
        .iter()
        .all(|required| present_files.contains(required))
}
