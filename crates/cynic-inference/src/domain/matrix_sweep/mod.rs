pub mod concurrency;
pub mod empirical_store;
pub mod memory_estimator;
pub mod stream_metrics;
pub mod types;

pub use concurrency::assess_coresident_topology;
pub use memory_estimator::{
    ModelArchDescriptor, can_admit_model_configuration,
};
pub use stream_metrics::compute_stream_dynamics;
pub use types::{
    BenchmarkScenario, HardwareBudget, ModelConcurrencySlots, StreamDynamicsReport,
    TopologyAssessment,
};
pub use empirical_store::{
    BenchmarkProvenance, ConstraintFilter, EMPIRICAL_LEDGER_SCHEMA, EmpiricalProfileEntry,
    ModelEmpiricalScorecard, OomSafetyStatus, OptimizationGoal, aggregate_model_scorecard,
    solve_optimal_configuration,
};
