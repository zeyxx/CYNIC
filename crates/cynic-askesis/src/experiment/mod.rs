//! Provider-agnostic experimental pipelines with deterministic read-back proof.

pub mod plan;
pub mod port;
pub mod runner;
pub mod types;
pub mod wordle;

pub use plan::plan_trials;
pub use runner::run_wordle_episode;
pub use types::{ExperimentSpec, TrialPlan, WordleEpisodeProof};

