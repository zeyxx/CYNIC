pub mod layers;
pub mod planner;
pub mod proofs;
pub mod types;

pub use layers::runtime_execution_layers;
pub use planner::{
    build_runtime_execution_plan_from_gates, runtime_initial_observed_gates, runtime_missing_gates,
};
pub use proofs::{
    InferenceStatusProof, RuntimeProof, RuntimeProofBundle, RuntimeProofEvidence,
    RuntimeProofPersistenceRecord, runtime_proof_persistence_record,
};
pub use types::{
    RUNTIME_EXECUTION_PLAN_SCHEMA, RUNTIME_EXECUTION_SUITE_SCHEMA, RUNTIME_PROOF_BUNDLE_SCHEMA,
    RUNTIME_PROOF_PERSISTENCE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeExecutionCommand,
    RuntimeExecutionLayer, RuntimeExecutionPlan, RuntimeExecutionSuite,
    RuntimePlannedExecutionLayer, RuntimeRegistrySummary, RuntimeSharedResponsibility,
    runtime_shared_responsibility,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_names_are_stable_contracts() {
        assert_eq!(
            RUNTIME_EXECUTION_SUITE_SCHEMA,
            "cynic.runtime_execution_suite.v1"
        );
        assert_eq!(
            RUNTIME_EXECUTION_PLAN_SCHEMA,
            "cynic.runtime_execution_plan.v1"
        );
    }
}
