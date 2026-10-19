pub mod concurrency;
pub mod debate;
pub mod empirical_store;
pub mod matrix_live_runner;
pub mod matrix_sweep_executor;
pub mod memory_estimator;
pub mod pareto;
pub mod port;
pub mod stream_metrics;
pub mod types;

pub use concurrency::assess_coresident_topology;
pub use empirical_store::{
    BenchmarkProvenance, ConstraintFilter, EMPIRICAL_LEDGER_SCHEMA, EmpiricalProfileEntry,
    ModelEmpiricalScorecard, OomSafetyStatus, OptimizationGoal, aggregate_model_scorecard,
    solve_optimal_configuration,
};
pub use memory_estimator::{ModelArchDescriptor, can_admit_model_configuration};
pub use stream_metrics::compute_stream_dynamics;
pub use types::{
    BenchmarkScenario, HardwareBudget, ModelConcurrencySlots, StreamDynamicsReport,
    TopologyAssessment,
};
