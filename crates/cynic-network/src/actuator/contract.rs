//! Network actuator contract constants and pure helpers.
//!
//! Clean-room offloaded from `cynic-node` per Foundation §481 (SoC).
//! Runtime-layout-dependent values stay in `cynic-node`; this module holds
//! only pure constants and pure functions.

pub const NETWORK_ACTUATOR_SUDOERS_SUBJECT: &str = "cynic";
pub const NETWORK_ACTUATOR_POLICY_FILENAME: &str = "network-actuator.policy.json";
pub const NETWORK_ACTUATOR_MANIFEST_FILENAME: &str = "manifest.json";
pub const NETWORK_ACTUATOR_MEMBRANE_CONTRACT: &str = "single cynic binary admitted by the privilege membrane; no helper binary; bounded network-actuate only";

pub fn bundle_required_files() -> [&'static str; 2] {
    [
        NETWORK_ACTUATOR_MANIFEST_FILENAME,
        NETWORK_ACTUATOR_POLICY_FILENAME,
    ]
}

pub fn membrane_admission(executor_path: &str) -> String {
    format!("{} privilege exec network-actuate", executor_path)
}
