// The shared detector is designed to be included by binaries that use different
// halves of it. `dead_code = "deny"` at workspace level otherwise fails the build
// for the unused half, which means this ratchet would not execute at all.
#![allow(dead_code)]

//! Orphan-source ratchet for `cynic-mcp`.
//!
//! Shares one detector with every other crate (`tests/support/orphan_sources.rs`)
//! so the reachability rules are defined once. See that file for why `mod`,
//! `#[path]`, `include!`, `Cargo.toml` targets and `src/bin/*.rs` must all be followed.
//!
//! Generated from the detector's own output, not from a separate resolver: a hand
//! count and the instrument disagree, and the instrument is the authority.
//!
//! The list is measured, not assumed, and it may only shrink. Nothing compiles
//! these files, so any contract they appear to enforce is inert.

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;

/// Sources currently unreachable from this crate's compilation roots.
const QUARANTINED: &[&str] = &[
    "handler.rs",
    "helpers.rs",
    "helpers_split/allow_without_why_contract.rs",
    "helpers_split/api_layer_alignment_contract.rs",
    "helpers_split/api_topology_contract.rs",
    "helpers_split/command_surfaces_contract.rs",
    "helpers_split/coord.rs",
    "helpers_split/fleet.rs",
    "helpers_split/hardcoded_paths_contract.rs",
    "helpers_split/infra_consumption_contract.rs",
    "helpers_split/infrastructure_contract.rs",
    "helpers_split/ip_contract.rs",
    "helpers_split/lifecycle.rs",
    "helpers_split/mcp_surface.rs",
    "helpers_split/misc.rs",
    "helpers_split/network.rs",
    "helpers_split/openapi_route_contract.rs",
    "helpers_split/pipeline_functions_contract.rs",
    "helpers_split/proxmox.rs",
    "helpers_split/registry.rs",
    "helpers_split/reqwest_usage_contract.rs",
    "helpers_split/secrets_contract.rs",
    "helpers_split/storage.rs",
    "helpers_split/unlogged_ok_contract.rs",
    "infra_provision_patch.rs",
    "kernel_discovery.rs",
    "local_tools.rs",
    "project_root.rs",
    "proxy.rs",
    "routers/coord.rs",
    "routers/docker.rs",
    "routers/eval.rs",
    "routers/forward.rs",
    "routers/local.rs",
    "types.rs",
    "types_split/coord.rs",
    "types_split/fleet.rs",
    "types_split/identity.rs",
    "types_split/infra.rs",
    "types_split/network.rs",
    "types_split/storage.rs",
    "types_split/task.rs",];

#[test]
fn orphan_sources_match_quarantine() {
    orphan_sources::assert_orphan_sources_match("cynic-mcp", QUARANTINED);
}
