//! Epoch-scoped orchestration for framework bootstrap phases.
//!
//! This module deliberately owns only the phase graph and its proof contract.
//! Provider transports, artifact publication, and service activation remain
//! injected adapters in their existing surfaces.

use serde::{Deserialize, Serialize};

use cynic_node_protocol::{DeploymentEpoch, DeploymentExecutionClass, DeploymentPhase, DeploymentPhaseObservation, DeploymentTransition, derive_deployment_epoch, phase_execution_class, plan_deployment_transition};

pub const FRAMEWORK_EPOCH_PLAN_SCHEMA: &str = "cynic.framework_bootstrap.epoch_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkEpochPhase {
    pub phase: DeploymentPhase,
    pub execution_class: DeploymentExecutionClass,
    pub owner: String,
    pub required_proofs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkEpochPlan {
    pub schema: String,
    pub epoch: DeploymentEpoch,
    pub status: String,
    pub phases: Vec<FrameworkEpochPhase>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

pub fn plan_framework_epoch(
    source_commit: &str,
    desired_state_digest: &str,
    target_scope: &str,
    created_at_unix_ms: u64,
) -> Result<FrameworkEpochPlan, String> {
    let epoch = derive_deployment_epoch(
        source_commit,
        desired_state_digest,
        target_scope,
        created_at_unix_ms,
    )?;
    Ok(FrameworkEpochPlan {
        schema: FRAMEWORK_EPOCH_PLAN_SCHEMA.to_string(),
        epoch,
        status: "planned".to_string(),
        phases: phase_catalog(),
        blockers: Vec::new(),
        residual_unknowns: vec![
            "phase observations are required before any transition may advance".to_string(),
            "provider transport and node activation remain adapter-owned".to_string(),
        ],
    })
}

pub fn advance_framework_epoch(
    epoch: &DeploymentEpoch,
    current: DeploymentPhase,
    observation: &DeploymentPhaseObservation,
) -> DeploymentTransition {
    plan_deployment_transition(epoch, current, observation)
}

fn phase_catalog() -> Vec<FrameworkEpochPhase> {
    vec![
        (
            DeploymentPhase::Proposed,
            "cynic-protocol::deployment_epoch",
            &["source_commit_read_back", "desired_state_digest_read_back"][..],
        ),
        (
            DeploymentPhase::Admitted,
            "cynic-host::framework_bootstrap::lifecycle",
            &["admission_read_back", "policy_digest_read_back"][..],
        ),
        (
            DeploymentPhase::Building,
            "cynic-node::build_scheduler",
            &[
                "durable_workload_delivery_read_back",
                "build_grant_read_back",
            ][..],
        ),
        (
            DeploymentPhase::Built,
            "cynic-node::build_scheduler",
            &["binary_build_observed", "completion_evidence_read_back"][..],
        ),
        (
            DeploymentPhase::Staging,
            "cynic-node::runtime_artifact_transport",
            &[
                "artifact_store_health_read_back",
                "artifact_transport_read_back",
            ][..],
        ),
        (
            DeploymentPhase::Staged,
            "cynic-host::framework_bootstrap::runtime_binding",
            &["runtime_seed_read_back", "registry_generation_read_back"][..],
        ),
        (
            DeploymentPhase::Canary,
            "cynic-host::service_reconcile",
            &[
                "node_service_active_read_back",
                "mcp_compatibility_read_back",
            ][..],
        ),
        (
            DeploymentPhase::RollingOut,
            "cynic-node::fleet_scheduler",
            &["fleet_rollout_read_back", "lease_fence_read_back"][..],
        ),
        (
            DeploymentPhase::Verifying,
            "cynic-host::framework_bootstrap::lifecycle",
            &[
                "terminal_registry_read_back",
                "terminal_runtime_health_read_back",
            ][..],
        ),
        (
            DeploymentPhase::Completed,
            "cynic-protocol::deployment_epoch",
            &["complete_epoch_read_back"][..],
        ),
    ]
    .into_iter()
    .map(|(phase, owner, proofs)| FrameworkEpochPhase {
        phase,
        execution_class: phase_execution_class(phase),
        owner: owner.to_string(),
        required_proofs: proofs.iter().map(ToString::to_string).collect(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_plan_is_stable_and_explicitly_unknown_until_observed() {
        let plan = plan_framework_epoch("commit-1", "sha256:desired", "fleet", 1).unwrap();
        assert_eq!(plan.status, "planned");
        assert_eq!(plan.phases.len(), 10);
        assert!(!plan.residual_unknowns.is_empty());
    }

    #[test]
    fn stale_observation_does_not_advance_epoch() {
        let plan = plan_framework_epoch("commit-1", "sha256:desired", "fleet", 1).unwrap();
        let observation = DeploymentPhaseObservation {
            schema: cynic_node_protocol::DEPLOYMENT_TRANSITION_SCHEMA.to_string(),
            epoch_id: plan.epoch.epoch_id.clone(),
            phase: DeploymentPhase::Proposed,
            ready: true,
            pending: true,
            blockers: Vec::new(),
            evidence_refs: Vec::new(),
        };
        let transition =
            advance_framework_epoch(&plan.epoch, DeploymentPhase::Proposed, &observation);
        assert_eq!(transition.status, "blocked");
        assert_eq!(transition.to, DeploymentPhase::Proposed);
    }
}
