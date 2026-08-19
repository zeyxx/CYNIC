//! Orphan-source ratchet for `cynic-cli`.
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
    "agent_harness_adapter.rs",
    "autoresearch_surface/runner.rs",
    "bootstrap_surface/framework/framework_epoch.rs",
    "bootstrap_surface/host_os_health_surface.rs",
    "dev_surface/build_admission.rs",
    "fleet_build_runner.rs",
    "hermes_kanban_adapter.rs",
    "hermes_runtime_surface.rs",
    "inference_benchmark.rs",
    "judgment_surface.rs",
    "judgment_surface/render.rs",
    "judgment_surface/scan.rs",
    "judgment_surface/types.rs",
    "lifecycle_surface/workload.rs",
    "runtime_surface/adoption.rs",
    "runtime_surface/agents.rs",
    "runtime_surface/epoch.rs",
    "runtime_surface/harness_matrix.rs",
    "runtime_surface/hermes_run.rs",
    "runtime_surface/inference_surface/fleet.rs",
    "runtime_surface/inference_surface/host_disk_capacity.rs",
    "runtime_surface/mcp.rs",
    "runtime_surface/process_convergence.rs",
    "runtime_surface/storage_capabilities.rs",
    "runtime_surface/storage_census.rs",
];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-cli", QUARANTINED);
}
