//! Privileged execution policy: declarative bounded helpers projected into
//! root-owned executors + generated sudoers membranes. See
//! `pve_helper` for the Proxmox host case and the cynic-privileged-execution
//! authority model (HumanAdmin bootstrap -> RuntimeMachineIdentity use).

pub mod pve_helper;

pub use pve_helper::{
    DEFAULT_POLICY_PATH, PVE_HELPER_BUNDLE_MANIFEST_SCHEMA, PVE_HELPER_POLICY_SCHEMA,
    PveHelperBundleManifest, PveHelperCapability, PveHelperPolicy, PvePolicyError,
    pve_helper_bundle_manifest, pve_helper_install_commands, pve_helper_read_back_commands,
};
