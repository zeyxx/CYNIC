#![cfg_attr(test, allow(dead_code, clippy::expect_used))]
#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    clippy::collapsible_if,
    clippy::implicit_clone,
    clippy::needless_pass_by_value,
    clippy::single_match,
    clippy::unused_self,
    missing_debug_implementations,
    async_fn_in_trait,
    clippy::too_many_arguments,
    clippy::redundant_clone,
    clippy::manual_contains,
    clippy::needless_borrow,
    clippy::print_stdout
)]

pub mod adapters;
pub mod finance_adapters;
pub(crate) mod announce;
mod build_attestation;
pub mod builder_activation;
pub mod config;
pub(crate) mod config_watch;
pub mod dns_activation;
pub mod dns_activation_status;
pub mod dns_bundle;
mod dns_contract;
pub mod dns_plan;
pub mod domain;
pub mod fleet_inventory;
pub mod fleet_observability;
pub mod inference_node_ops;
pub mod inference_operations;
pub mod mesh_registry;
pub mod network_activation_chain;
pub mod network_actuator;
mod network_actuator_contract;
pub mod network_actuator_status;
mod network_command_surface;
mod network_diagnostic_surface;
pub mod network_evidence;
mod network_host_substrate;
pub mod network_mesh_apply_plan;
pub mod network_mesh_bundle;
pub mod network_mesh_plan;
pub mod network_repair;
pub mod network_root_policy;
pub mod network_runtime_proof;
pub mod network_runtime_status;
pub mod network_shadow_proof;
mod network_surface_inventory;
pub mod node_control_runtime;
pub mod observe;
pub mod peer_commands;
pub mod ports;
pub mod provider_fallback_hosts;
pub mod provider_fallback_status;
pub mod runtime_supervision;
pub mod shutdown;
pub(crate) mod source_change_transaction;
pub(crate) mod supervise;
pub(crate) mod system_probe;
pub mod usecases;
pub mod verify;
pub mod watchdog;
pub mod websocket;


// Cross-crate surface, explicit.
//
// These replace `pub use facade::*;`. A glob exported 13 functions; 8 had zero
// consumers anywhere, and a glob cannot tell a used name from a dead one, so the
// dead ones were indistinguishable from the API. Each name below has a measured
// consumer, and the two that are already reachable through `pub mod usecases` /
// `pub mod adapters` are re-exported rather than duplicated.
//
// The CLI needs these because cynic-node's network modules are `mod`, not
// `pub mod`. Widening those is slice 2, where the surfaces move out.
pub use adapters::peer_storage::ingress::execute_peer_command;
pub use network_runtime_status::build_network_runtime_status_value;
