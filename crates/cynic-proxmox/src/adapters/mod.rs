//! Proxmox adapters — LXC clone, disk extension, substrate observation.
//!
//! Every adapter drives the hypervisor through `cynic-lifecycle`'s
//! `ArgvRunner` (typed argv, never raw shell) or the PVE REST API.

pub mod agent_host_observer;
pub mod proxmox_lxc_create;
pub mod ed25519_credential_lease_verifier;
pub mod proxmox_clone;
pub mod proxmox_control;
pub mod proxmox_disk_extension;
pub mod proxmox_lxc_compute_substrate;
pub mod proxmox_substrate;

pub use agent_host_observer::{
    AgentHostObservationPort, ProxmoxRestAgentHostObserver, ProxmoxSshAgentHostObserver,
};
pub use proxmox_lxc_create::{
    build_lxc_create_body, validate_ca_pem, LxcCreateReadBack, LxcCreateReceipt,
    LxcCreateRollback, ProxmoxRestLxcCreator, LXC_CREATE_RECEIPT_SCHEMA,
};

pub use ed25519_credential_lease_verifier::Ed25519CredentialLeaseVerifier;
pub use proxmox_clone::{LxcCloneObservation, LxcCloneOutcome, ProxmoxSshCloneAdapter};
pub use proxmox_control::{
    ProxmoxControlAdapter, ProxmoxControlConfiguration, ProxmoxControlStatus,
};
pub use proxmox_disk_extension::ProxmoxSshDiskExtensionAdapter;
pub use proxmox_lxc_compute_substrate::{ProxmoxLxcSubstrateAdapter, ProxmoxLxcSubstrateError};
pub use proxmox_substrate::ProxmoxSshSubstrateAdapter;
