//! Bottom-up, reproducible agent/inference benchmark harness.
//!
//! This layer owns schemas and pure planning/policy only.  Process spawning,
//! HTTP clients, and host metrics are injected later through adapters.

mod baseline;
mod plan;
mod policy;
mod ports;
mod report;
mod runner;
mod types;

pub mod benchmark_policy;

pub use baseline::{
    BENCHMARK_BASELINE_ASSESSMENT_SCHEMA, BENCHMARK_BASELINE_SCHEMA, BenchmarkBaseline,
    BenchmarkBaselineAssessment, assess_baseline,
};
pub use benchmark_policy::{
    AcceptanceBaselineSpec, BenchmarkPolicySpec, ExecutionParameterSweepSpec, KvCacheSweepSpec,
    UseCaseSpec, generate_plan_from_policy, load_benchmark_policy,
};
pub use plan::{BenchmarkPlan, ScenarioPlan};
pub use policy::{StopDecision, evaluate_sample};
pub use ports::{
    CoordinationPort, CoordinationReceipt, HarnessExecutor, HarnessObserver, InferenceReply,
};
pub use report::{
    BENCHMARK_MATRIX_SCHEMA, BenchmarkReport, BenchmarkSummary, aggregate_reports, encode_report,
};
pub use runner::{run_scenario, run_scenario_with_coordination};
pub use types::{canonicalize_dog_runtime_id, canonicalize_hermes_runtime_id};
pub use types::{
    AgentHarnessReport, BENCHMARK_SCHEMA, CycleEnvelope, HarnessSample, MemorySample,
    ModelParameters, ModelSpec, RuntimeKind, RuntimeLayer, RuntimePlanSummary, ScenarioSpec,
    TaskKind, TaskSpec, Thresholds,
};
