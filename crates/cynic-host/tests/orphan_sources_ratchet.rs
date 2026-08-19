//! Orphan-source ratchet for `cynic-host`.
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
    "adapters/host_exec_disk.rs",
    "adapters/proxmox_clone.rs",
    "audit_engine/mod.rs",
    "audit_engine/rules.rs",
    "audit_engine/scanner.rs",
    "container_runtime/local.rs",
    "container_runtime/mod.rs",
    "container_runtime/normalize.rs",
    "container_runtime/policy.rs",
    "container_runtime/port.rs",
    "container_runtime/types.rs",
    "coordination_lease_store/mod.rs",
    "detection_engine/hardware.rs",
    "detection_engine/mod.rs",
    "detection_engine/provenance.rs",
    "detection_engine/supervisor.rs",
    "framework_bootstrap/activation.rs",
    "framework_bootstrap/admission.rs",
    "framework_bootstrap/enrollment.rs",
    "host_hardening/apply.rs",
    "host_hardening/mod.rs",
    "host_hardening/observe.rs",
    "host_hardening/reconcile.rs",
    "host_hardening/types.rs",
    "local_agent_session.rs",
    "runtime_takeover_activation.rs",
    "source_control/commit_scope.rs",
    "storage_census/growth.rs",
    "storage_census/local.rs",
    "storage_census/mod.rs",
    "storage_census/policy.rs",
    "storage_census/port.rs",
    "storage_census/process.rs",
    "storage_census/types.rs",
    "update_lifecycle/repair_loop.rs",
];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-host", QUARANTINED);
}
