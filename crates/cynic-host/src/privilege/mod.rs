//! Privileged execution policy: declarative bounded helpers projected into
//! root-owned executors + generated sudoers membranes. See
//! `pve_helper` for the Proxmox host case and the cynic-privileged-execution
//! authority model (HumanAdmin bootstrap -> RuntimeMachineIdentity use).

pub mod capability_catalog;
pub mod pve_helper;
pub mod transport_caps;

pub use capability_catalog::{
    CapabilityLoadError, CapabilityView, DEFAULT_CATALOG_PATH, DEFAULT_INSTALLED_PATH,
    catalog_path_from_env, installed_path_from_env,
};
pub use pve_helper::{
    DEFAULT_POLICY_PATH, PVE_HELPER_BUNDLE_MANIFEST_SCHEMA, PVE_HELPER_POLICY_SCHEMA,
    PveHelperBundleManifest, PveHelperPolicy, PvePolicyError, pve_helper_bundle_manifest,
    pve_helper_install_commands, pve_helper_read_back_commands,
};
pub use transport_caps::{
    AuthorityClass, DEFAULT_TRANSPORT_POLICY_PATH, TRANSPORT_CAPABILITIES_SCHEMA,
    TransportCapabilities, TransportCapability, TransportCapabilityParam, TransportPolicyError,
};
// `ParamKind` is re-exported here for transport callers; it lives in
// cynic-protocol (single source of truth) and is NOT redefined here.
pub use cynic_protocol::privilege::ParamKind;
