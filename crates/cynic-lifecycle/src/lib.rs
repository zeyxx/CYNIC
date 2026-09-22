//! Update lifecycle: harness phases as promotion membrane + ops repair.
//!
//! Layering (not a monolith):
//! - [`types`] — report/state vocabulary
//! - [`time`] — clock helper
//! - [`port`] — [`ArgvRunner`] (domain never owns `Command::new("cynic")`)
//! - [`policy`] — phase → gates/mutations (pure tables)
//! - [`state_store`] — JSON persistence paths
//! - [`plan`] — status + planned actions
//! - [`gate`] / [`promote`] — gate execute + promote
//! - [`observe`] — ops findings from runner JSON
//! - [`repair`] / [`apply`] — phase-filtered mutations
//!
//! CLI injects [`LocalArgvRunner`]; tests inject [`MapArgvRunner`].

mod apply;
pub mod domain;
pub mod gate;
pub mod harness_context;
pub mod kernel_release;
mod observe;
mod plan;
mod policy;
mod port;
pub mod ports;
mod promote;
mod repair;
mod state_store;
mod time;
mod types;

pub use apply::apply_phase;
pub use gate::run_gates;
pub use kernel_release::{
    KernelReleaseReport, KernelReleaseSpec, reconcile_kernel_release,
    reconcile_runtime_artifact_release,
};
pub use observe::ops_report;
pub use plan::{plan_report, promote_eligibility, status_report};
pub use policy::phase_policy;
pub use port::{ArgvOutput, ArgvRunner, LocalArgvRunner, MapArgvRunner, parse_json, split_command};
pub use promote::promote;
pub use repair::{repair_apply, repair_plan};
pub use state_store::{default_state_path, load_state, save_state};
pub use types::*;

use std::path::Path;

/// Convenience: status with default local runner (no subprocess for pure status).
pub fn status_report_default(state_path: &Path) -> LifecycleStatusReport {
    status_report(state_path)
}

/// Convenience wrappers that inject [`LocalArgvRunner`] for thin CLI call sites.
pub fn ops_report_local(state_path: &Path) -> OpsReport {
    ops_report(state_path, &LocalArgvRunner)
}

pub fn run_gates_local(state_path: &Path, execute: bool) -> LifecycleGateReport {
    run_gates(state_path, execute, &LocalArgvRunner)
}

pub fn repair_plan_local(state_path: &Path) -> RepairReport {
    repair_plan(state_path, &LocalArgvRunner)
}

pub fn repair_apply_local(state_path: &Path, yes: bool) -> RepairReport {
    repair_apply(state_path, yes, &LocalArgvRunner)
}

pub fn apply_phase_local(state_path: &Path, yes: bool) -> ApplyReport {
    apply_phase(state_path, yes, &LocalArgvRunner)
}
