pub mod adapters;
pub mod bucket_signatures;
pub mod capacity;
pub mod domain;
pub mod ecosystem_package;
pub mod agent_access;
pub mod bootstrap_chain;
pub mod bootstrap_host_plan;
pub mod bridge;

pub mod clock;
pub mod command_surface;
pub mod conformance;
pub mod rate_classify_scan;
// The store two integration tests already import. It was never declared, so the
// coordination lease enforcement it implements has been inert: nothing compiled
// it, and both test suites failed at E0432 rather than exercising it.
pub mod coordination_lease_store;
pub mod crypto;
pub mod infra;
pub mod endpoint_observation;
pub mod execution;

pub mod federation_runtime;
pub mod filesystem_observation;
pub mod framework_bootstrap;
pub mod gc_surfaces;
pub mod hardware_baseline;
pub mod hardware_baseline_apply;
pub mod hardware_baseline_reconcile;
pub mod hardware_conformance;
pub mod harness_progression;
pub mod host_bootstrap_evidence;
pub mod host_exec_cli;
pub mod host_exec_policy;
pub mod host_substrate;
mod identity_database;
pub mod install_contract;
pub mod kernel_proof_persistence;
pub mod lab;
pub mod lease_manager;
pub mod forge_task_contract;
pub use forge_task_contract::{validate_forge_lifecycle_contract, validate_repo_affecting_hermes_task};
pub mod mcp_tool;
pub mod node_identity_workflow;
pub mod node_substrate;
pub mod one_shot_authority;
pub mod os_health;
pub mod peer_resolver_mesh;
pub mod peer_validator;
pub mod plan_verifier;
pub mod ports;
pub mod project_root;
pub mod pre_os_control;
pub mod principal_prepare;
pub mod privilege;
pub mod process_convergence;
pub mod profiles;
pub mod registry;
pub mod runtime_context;
pub mod runtime_execution_context;
pub mod runtime_metrics;
pub mod runtime_network_preflight;
pub mod runtime_proof;
pub mod runtime_registry;
pub mod runtime_resolver_plan;
mod runtime_secret_install;
pub mod runtime_seed;
pub mod secret_presence;
pub mod service_catalog;
pub mod service_supervisor_observation;
pub mod space_gc;

pub use cynic_ssh::config as ssh_config;
pub use cynic_ssh::trust as ssh_trust;
pub mod storage_credential_sync;
pub(crate) mod storage_identity;
pub mod storage_primary;
pub mod transport;
pub mod usecases;

#[cfg(test)]
pub(crate) mod test_support;

pub fn sha256_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
pub mod emergence_runtime;
pub mod local_autonomy_runtime;
