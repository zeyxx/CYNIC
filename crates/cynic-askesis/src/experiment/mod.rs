//! Provider-agnostic experimental pipelines with deterministic read-back proof.

pub mod plan;
pub mod port;
pub mod runner;
pub mod simulator;
pub mod types;

pub use plan::plan_trials;
pub use runner::run_reconciliation_episode;
pub use types::{ExperimentSpec, ReconciliationProof, TrialPlan};
