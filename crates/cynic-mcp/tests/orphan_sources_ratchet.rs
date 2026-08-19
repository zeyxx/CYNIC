//! Orphan-source ratchet for `cynic-mcp`.
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
    "handler_tests.rs",
    "handler_tests/allow_without_why_contract_tests.rs",
    "handler_tests/api_layer_alignment_contract_tests.rs",
    "handler_tests/api_topology_contract_tests.rs",
    "handler_tests/command_surfaces_contract_tests.rs",
    "handler_tests/coordination_profile_tests.rs",
    "handler_tests/hardcoded_paths_contract_tests.rs",
    "handler_tests/http_query_tests.rs",
    "handler_tests/infra_consumption_contract_tests.rs",
    "handler_tests/infrastructure_contract_tests.rs",
    "handler_tests/infrastructure_mcp_contract_tests.rs",
    "handler_tests/ip_contract_tests.rs",
    "handler_tests/lifecycle_dispatch_tests.rs",
    "handler_tests/mcp_surface_tests.rs",
    "handler_tests/observe_tests.rs",
    "handler_tests/openapi_route_contract_tests.rs",
    "handler_tests/pipeline_functions_contract_tests.rs",
    "handler_tests/reqwest_usage_contract_tests.rs",
    "handler_tests/secrets_contract_tests.rs",
    "handler_tests/solana_sop_tests.rs",
    "handler_tests/unlogged_ok_contract_tests.rs",
    "helpers_split/api_topology_contract/findings.rs",
    "helpers_split/api_topology_contract/load.rs",
    "helpers_split/api_topology_contract/operation_geometry.rs",
    "helpers_split/api_topology_contract/paths.rs",
    "helpers_split/api_topology_contract/public_discovery.rs",
    "infra_provision_patch.rs",
    "kernel_discovery.rs",
    "local_tools_split/docker.rs",
    "local_tools_split/fleet_build_runner.rs",
    "local_tools_split/git.rs",
    "local_tools_split/mod.rs",
    "local_tools_split/shared.rs",
    "local_tools_split/source_control.rs",
    "routers/forward/contracts.rs",
    "routers/forward/health.rs",
    "routers/forward/infer.rs",
    "routers/forward/legacy_tasks.rs",
    "routers/forward/observe.rs",
    "types_split/validation.rs",
];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-mcp", QUARANTINED);
}
