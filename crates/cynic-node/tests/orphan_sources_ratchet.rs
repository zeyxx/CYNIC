//! Orphan-source ratchet for `cynic-node`.
//!
//! Shares one detector with every other crate (`tests/support/orphan_sources.rs`)
//! so reachability rules are defined once. See that file for why `mod`,
//! `#[path]`, `include!`, and `Cargo.toml` targets must all be followed.
//!
//! The list below is measured, not assumed: every entry was confirmed
//! unreachable by the shared detector, and the detector itself was validated by
//! probe — removing a file and rebuilding, so a live file cannot be recorded as
//! dead. The test fails in both directions: a new orphan is rejected, and a
//! repaired file must be removed from the list.

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;

/// Sources currently unreachable from this crate's roots.
///
/// Quarantined as explicit data with the debt visible, NOT tolerated: nothing
/// compiles these files, so any contract they claim to enforce is inert. The
/// list may only shrink.
const QUARANTINED: &[&str] = &[
    "adapters/local_node_attestation.rs",
    "adapters/local_observation_journal.rs",
    "adapters/local_workload_signer.rs",
    "adapters/registry_mesh_state_provider.rs",
    "adapters/runtime_proof_http_fetcher.rs",
    "build_executor.rs",
    "fleet_inventory/kubernetes.rs",
    "fleet_inventory/power.rs",
    "fleet_inventory/providers.rs",
    "fleet_inventory/proxmox.rs",
    "fleet_inventory_tests.rs",
    "fleet_placement.rs",
    "lifecycle.rs",
    "local_agent_session.rs",
    "mcp_runtime_observation.rs",
    "network_mesh_apply_plan/canary_admission.rs",
    "ports/agent_workload_submission_port.rs",
    "ports/federated_bootstrap_read_back_port.rs",
    "ports/node_attestation_port.rs",
    "ports/observation_journal_port.rs",
    "ports/runtime_proof_fetcher_port.rs",
    "ports/runtime_release_declaration_port.rs",
    "ports/solana_pay_port.rs",
    "ports/workload_signer_port.rs",
    "takeover_activation.rs",
    "takeover_activation/store.rs",
    "usecases/build_scheduler/conformance.rs",
    "usecases/federated_bootstrap_uc.rs",
    "usecases/federated_mesh_probe_uc.rs",
    "usecases/federated_mesh_resolver.rs",
    "usecases/federated_task_dispatch_uc.rs",
    "usecases/kernel_spof_observe_uc.rs",
    "usecases/node_network_recovery_uc.rs",
    "usecases/runtime_release_declaration_uc.rs",
    "usecases/source_change_scheduler.rs",
    "usecases/source_change_submission_uc.rs",
    "usecases/zeroclaw_allowance_uc.rs",
    "watchdog/peer_handshake.rs",
];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-node", QUARANTINED);
}
