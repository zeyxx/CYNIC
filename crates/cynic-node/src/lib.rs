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
pub(crate) mod announce;
mod build_attestation;
pub mod builder_activation;
pub mod cli;
pub mod cli_dispatch;
pub mod config;
pub(crate) mod config_watch;
mod dns_activation;
mod dns_activation_status;
mod dns_bundle;
mod dns_contract;
mod dns_plan;
pub mod domain;
pub mod facade;
pub mod fleet_inventory;
pub mod fleet_observability;
pub mod inference_node_ops;
pub mod inference_operations;
pub mod mesh_registry;
mod network_activation_chain;
mod network_actuator;
mod network_actuator_contract;
mod network_actuator_status;
mod network_command_surface;
mod network_diagnostic_surface;
mod network_evidence;
mod network_host_substrate;
mod network_mesh_apply_plan;
mod network_mesh_bundle;
mod network_mesh_plan;
mod network_repair;
mod network_root_policy;
mod network_runtime_proof;
mod network_runtime_status;
mod network_shadow_proof;
mod network_surface_inventory;
pub mod node_control_runtime;
mod observe;
pub mod ports;
mod provider_fallback_hosts;
mod provider_fallback_status;
pub mod runtime_supervision;
pub mod shutdown;
pub(crate) mod supervise;
pub(crate) mod system_probe;
pub mod usecases;
pub mod verify;
pub mod watchdog;
pub mod websocket;

pub use cli::NetworkCommands;
pub use facade::*;
