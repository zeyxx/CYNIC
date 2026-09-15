use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::AGENT_WORKLOAD_ENVELOPE_SCHEMA;

pub const AGENT_CONSTITUTION_STATUS_SCHEMA: &str = "cynic.agent_constitution_status.v1";
pub const AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA: &str =
    "cynic.agent_execution_runtime_observation.v1";

pub const AGENT_EXECUTION_REQUIRED_RUNTIME_CAPABILITIES: &[&str] = &[
    "kernel_to_node_workload_delivery_read_back",
    "scheduler_claim_renewal",
    "scheduler_cancellation_delivery",
    "scheduler_crash_recovery",
    "node_owned_build_grant_machine_signer",
    "build_runner_signature_and_live_workload_verifier",
    "scheduler_to_build_runner_execution_transport",
    "build_completion_and_artifact_read_back",
    "node_workspace_observer_wiring_and_proof",
    "workspace_claim_store_wiring_and_conformance_proof",
    "bounded_source_change_apply_adapter",
    "post_apply_source_read_back",
];

const REQUIRED_INVARIANTS: &[&str] = &[
    "admitted_delegation_and_policy",
    "monotonic_capabilities_and_budgets",
    "durable_idempotency",
    "aggregate_sibling_budget_accounting",
    "lease_fencing_and_expiry",
    "isolated_workspace_and_hierarchical_source_zones",
    "content_addressed_source_preconditions",
    "durable_cancellation",
    "recovery_after_claimant_or_node_failure",
    "content_addressed_completion_evidence",
    "workload_derived_machine_build_authority",
];

const IMPLEMENTED_PROTOCOL_PRIMITIVES: &[&str] = &[
    "workload_admission",
    "content_addressed_bounded_argv",
    "content_addressed_semantic_action_input",
    "policy_action_binding",
    "dispatch_idempotency_admission",
    "lease_fenced_claim_and_reclaim",
    "action_budget_consumption",
    "cancellation_generation",
    "completion_admission",
    "pure_workspace_reconciliation",
    "hierarchical_source_zone_conflict_detection",
    "source_base_and_preimage_drift_rejection",
    "workspace_claim_intent_admission_and_digest_binding",
    "atomic_workspace_claim_store_port",
    "injected_read_only_local_workspace_observer",
    "portable_registry_workspace_layout_adapter",
    "surreal_atomic_hierarchical_workspace_claim_store",
    "workspace_claim_hard_expiry_and_stale_fence_rejection",
    "durable_agent_workload_store",
    "aggregate_sibling_budget_accounting",
    "durable_workload_cancellation",
    "completion_evidence_persistence",
    "canonical_build_action_expansion",
    "workload_derived_build_grant",
    "build_grant_revision_fence_and_cancellation_binding",
    "canonical_build_grant_signing_payload",
    "injected_build_grant_signer_and_verifier_ports",
    "ed25519_machine_build_grant_adapter",
    "append_only_build_grant_store",
    "idempotent_build_grant_reconciliation_and_read_back",
    "canonical_build_request_workload_composition",
    "kernel_durable_workload_delivery",
    "node_control_workload_ingress",
    "target_node_delivery_persistence_before_ack",
    "build_source_transport_and_revision_binding",
    "managed_repository_fetch_and_commit_read_back",
    "node_owned_build_scheduler_loop",
    "scheduler_claim_renewal_and_cancellation_monitor",
    "isolated_build_workspace_read_back",
    "scheduler_derived_machine_grant_after_action_reservation",
    "cancellable_argv_build_execution_adapter",
    "content_addressed_build_completion_proofs",
    "durable_completion_envelope_retention",
    "claimed_and_unclaimed_cancellation_acknowledgement",
];

const LEGACY_AGENT_TASK_GAPS: &[&str] = &[
    "stringly_typed_state",
    "no_idempotency_key",
    "no_delegation_or_policy_binding",
    "no_budget_accounting",
    "no_claim_expiry_or_fencing",
    "no_cancellation",
    "completion_evidence_not_persisted_with_task",
    "rest_completion_claims_as_constant_principal",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionRuntimeObservation {
    pub schema: String,
    #[serde(default)]
    pub observed_capabilities: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

impl AgentExecutionRuntimeObservation {
    pub fn unobserved() -> Self {
        Self {
            schema: AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA.to_string(),
            observed_capabilities: Vec::new(),
            evidence_refs: Vec::new(),
            unknowns: vec!["node_owned_agent_execution_observer_not_wired".to_string()],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentConstitutionRuntimeStatus {
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyAgentTaskStatus {
    pub classification: String,
    pub runtime_authority: bool,
    pub observed_gaps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConstitutionStatus {
    pub schema: String,
    pub owner: String,
    pub execution_contract: String,
    pub scheduler_owner: String,
    pub status: AgentConstitutionRuntimeStatus,
    pub required_invariants: Vec<String>,
    pub implemented_protocol_primitives: Vec<String>,
    pub required_runtime_capabilities: Vec<String>,
    pub observed_runtime_capabilities: Vec<String>,
    pub missing_runtime_capabilities: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
    pub legacy_agent_task: LegacyAgentTaskStatus,
    pub activation_gate: String,
}

pub fn reconcile_agent_execution_capabilities(
    observation: &AgentExecutionRuntimeObservation,
) -> AgentConstitutionStatus {
    let required = AGENT_EXECUTION_REQUIRED_RUNTIME_CAPABILITIES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut unknowns = observation.unknowns.clone();
    let observed = if observation.schema == AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA {
        observation
            .observed_capabilities
            .iter()
            .map(String::as_str)
            .filter(|capability| required.contains(capability))
            .collect::<BTreeSet<_>>()
    } else {
        unknowns.push("invalid_agent_execution_runtime_observation_schema".to_string());
        BTreeSet::new()
    };
    for capability in &observation.observed_capabilities {
        if !required.contains(capability.as_str()) {
            unknowns.push(format!(
                "unexpected_agent_execution_runtime_capability:{capability}"
            ));
        }
    }
    unknowns.sort();
    unknowns.dedup();

    let missing = required
        .difference(&observed)
        .map(|capability| (*capability).to_string())
        .collect::<Vec<_>>();
    let status = if missing.is_empty() && unknowns.is_empty() {
        AgentConstitutionRuntimeStatus::Active
    } else {
        AgentConstitutionRuntimeStatus::Inactive
    };

    AgentConstitutionStatus {
        schema: AGENT_CONSTITUTION_STATUS_SCHEMA.to_string(),
        owner: "cynic-protocol::agent_workload".to_string(),
        execution_contract: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
        scheduler_owner: "node_local".to_string(),
        status,
        required_invariants: strings(REQUIRED_INVARIANTS),
        implemented_protocol_primitives: strings(IMPLEMENTED_PROTOCOL_PRIMITIVES),
        required_runtime_capabilities: required
            .iter()
            .map(|capability| (*capability).to_string())
            .collect(),
        observed_runtime_capabilities: observed
            .iter()
            .map(|capability| (*capability).to_string())
            .collect(),
        missing_runtime_capabilities: missing,
        evidence_refs: observation.evidence_refs.clone(),
        unknowns,
        legacy_agent_task: LegacyAgentTaskStatus {
            classification: "coordination_message_queue_only".to_string(),
            runtime_authority: false,
            observed_gaps: strings(LEGACY_AGENT_TASK_GAPS),
        },
        activation_gate: "all required runtime capabilities must have node-owned durable read-back proof before status may become active".to_string(),
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unobserved_runtime_does_not_reclassify_protocol_primitives_as_missing() {
        let status =
            reconcile_agent_execution_capabilities(&AgentExecutionRuntimeObservation::unobserved());

        assert_eq!(status.status, AgentConstitutionRuntimeStatus::Inactive);
        assert!(
            status
                .implemented_protocol_primitives
                .contains(&"durable_agent_workload_store".to_string())
        );
        assert!(
            !status
                .missing_runtime_capabilities
                .contains(&"durable_agent_workload_store".to_string())
        );
        assert!(
            status
                .missing_runtime_capabilities
                .contains(&"scheduler_crash_recovery".to_string())
        );
    }

    #[test]
    fn only_complete_node_owned_read_back_activates_execution() {
        let observation = AgentExecutionRuntimeObservation {
            schema: AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA.to_string(),
            observed_capabilities: strings(AGENT_EXECUTION_REQUIRED_RUNTIME_CAPABILITIES),
            evidence_refs: vec!["proof:node-scheduler:sha256:abc".to_string()],
            unknowns: Vec::new(),
        };

        let status = reconcile_agent_execution_capabilities(&observation);

        assert_eq!(status.status, AgentConstitutionRuntimeStatus::Active);
        assert!(status.missing_runtime_capabilities.is_empty());
        assert_eq!(status.evidence_refs, observation.evidence_refs);
    }

    #[test]
    fn unknown_capability_cannot_amplify_runtime_authority() {
        let observation = AgentExecutionRuntimeObservation {
            schema: AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA.to_string(),
            observed_capabilities: vec!["omnipotent_scheduler".to_string()],
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };

        let status = reconcile_agent_execution_capabilities(&observation);

        assert_eq!(status.status, AgentConstitutionRuntimeStatus::Inactive);
        assert!(status.observed_runtime_capabilities.is_empty());
        assert_eq!(
            status.unknowns,
            vec!["unexpected_agent_execution_runtime_capability:omnipotent_scheduler"]
        );
    }
}
