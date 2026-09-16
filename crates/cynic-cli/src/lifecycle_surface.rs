//! Update lifecycle CLI — thin adapter over cynic-lifecycle.
//!
//! Business logic, policy tables, and observation live in the cynic-lifecycle
//! crate. This surface only: clap parse → inject LocalArgvRunner → print.
//!
//! ```text
//! cynic lifecycle status|plan|gate|promote
//! cynic lifecycle report          # end operational black box
//! cynic lifecycle repair [--yes]  # phase-filtered repairs
//! cynic lifecycle apply [--yes]   # phase-allowed mutations
//! cynic lifecycle kernel-release --spec <path> [--state <path>] [--yes]  # immutable artifact reconciliation
//! ```

use std::process::ExitCode;

use crate::lifecycle_surface_execute::{
    execute_apply, execute_evidence, execute_gate, execute_kernel_release, execute_plan,
    execute_promote, execute_repair, execute_report, execute_status,
};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum LifecycleCommand {
    /// Host-owned lifecycle evidence compatibility surface.
    ///
    /// The payload grammar remains in `cynic-host`; this adapter only removes
    /// the former standalone evidence binary from the installed runtime.
    Evidence {
        /// Evidence operation and its arguments. Use `--` before flags passed
        /// to the host-owned operation, for example `evidence runtime-paths -- --registry <path>`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Current phase, policy, promotion eligibility.
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Planned gates and allowed mutations for current phase.
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Run (or plan) promotion gates for the current phase.
    Gate {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Promote one phase forward after a green gate.
    Promote {
        #[arg(long)]
        json: bool,
        #[arg(long = "to")]
        to: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Operational observation report (findings + repair intents) — ends black box.
    Report {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Plan or apply repairs for open findings (phase-filtered).
    Repair {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Apply phase-allowed mutations (deps upgrade, inference discover, …).
    Apply {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Reconcile one immutable kernel artifact, verify identity, and record release PoH.
    KernelRelease {
        /// Path to a cynic.kernel_release.spec.v1 JSON desired-state file.
        #[arg(long)]
        spec: String,
        #[arg(long)]
        json: bool,
        /// Apply staging, atomic activation, restart, observation, and rollback policy.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
}

pub(crate) fn execute_lifecycle(command: LifecycleCommand) -> ExitCode {
    match command {
        LifecycleCommand::Evidence { args } => execute_evidence(args),
        LifecycleCommand::Status { json, state } => execute_status(json, state),
        LifecycleCommand::Plan { json, state } => execute_plan(json, state),
        LifecycleCommand::Gate { json, yes, state } => execute_gate(json, yes, state),
        LifecycleCommand::Promote {
            json,
            to,
            yes,
            state,
        } => execute_promote(json, to, yes, state),
        LifecycleCommand::Report { json, state } => execute_report(json, state),
        LifecycleCommand::Repair { json, yes, state } => execute_repair(json, yes, state),
        LifecycleCommand::Apply { json, yes, state } => execute_apply(json, yes, state),
        LifecycleCommand::KernelRelease {
            spec,
            json,
            yes,
            state,
        } => execute_kernel_release(spec, json, yes, state),
    }
}

