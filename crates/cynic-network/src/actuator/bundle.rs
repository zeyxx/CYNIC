//! Network actuator bundle helpers (pure).
//!
//! Offloaded from `cynic-node` per Foundation §481 (SoC). I/O-heavy bundle
//! creation remains in `cynic-node`.

pub fn bundle_required_files_present(required: &[&str], present: &[&str]) -> bool {
    required.iter().all(|file| present.contains(file))
}
