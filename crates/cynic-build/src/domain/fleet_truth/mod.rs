//! Build-runner selection + builder restart decision layer, hoisted out of
//! cynic-node into cynic-build.
//!
//! cynic-node keeps the broader fleet-truth reconcile surface (observation
//! reconcile, organ-alignment rules) in `cynic_node::domain::fleet_truth`;
//! the pure decision layer belongs here and cynic-node re-exports it.

pub mod build_runner_selection;
pub mod model;

pub use build_runner_selection::{SelectedBuildRunner, select_build_runner};
pub use model::*;

// Builder restart decisions are delegated to cynic_build::builder_restart.
// cynic_node::domain re-exports both decision layers for call-site compat.
