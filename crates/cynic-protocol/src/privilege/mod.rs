//! Privileged capability catalog — the single admission path.
//!
//! One catalog, one `admit_capability`, shared by every domain. The pure
//! types and the decision live in `catalog`; the installed-catalog read and
//! the exec live in adapters (`cynic-host`).
//!
//! This module exists because three implementations of the same `pct` argv
//! grew independently (`cynic-lxc::admit`, a shell script,
//! `proxmox_clone.rs`). Three implementations of one policy diverge silently.

mod catalog;

pub use catalog::{
    CAPABILITY_CATALOG_SCHEMA, INSTALLED_CATALOG_SCHEMA, Capability, CapabilityCatalog,
    CapabilityDecision, CapabilityParam, InstalledCapabilities, ParamKind, admit_capability,
    validate_install,
};
