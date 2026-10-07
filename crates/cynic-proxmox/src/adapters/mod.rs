//! Proxmox adapters — LXC clone, disk extension, substrate observation.
//!
//! Every adapter drives the hypervisor through `cynic-lifecycle`'s
//! `ArgvRunner` (typed argv, never raw shell) or the PVE REST API.

pub mod ed25519_credential_lease_verifier;
pub mod proxmox_clone;
pub mod proxmox_control;
pub mod proxmox_disk_extension;
pub mod proxmox_substrate;

pub use ed25519_credential_lease_verifier::Ed25519CredentialLeaseVerifier;
pub use proxmox_clone::{LxcCloneObservation, LxcCloneOutcome, ProxmoxSshCloneAdapter};
pub use proxmox_control::{ProxmoxControlAdapter, ProxmoxControlConfiguration, ProxmoxControlStatus};
pub use proxmox_disk_extension::ProxmoxSshDiskExtensionAdapter;
pub use proxmox_substrate::ProxmoxSshSubstrateAdapter;
