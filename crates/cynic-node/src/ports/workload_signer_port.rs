//! Phase 3.1b port — WorkloadSignerPort (AGENS.md §7 adapters disposable).
//!
//! Binds the existing cynic-protocol `AgentWorkloadEnvelope` (monotonic
//! budgets, lease fencing, cancellation) to the websocket federation dispatch
//! gate. `domain_dispatch::spawn_native_handler` was refusing native federation
//! mutation with blocker "runtime_federation_workload_signer_unavailable".
//! This port injects the signer so 2N+2 federation mutation can proceed once
//! Phase 3.1 (node-owned scheduler) is runtime-proven.
//!
//! AGENS.md §6: a decision must not be recalculated in multiple apply surfaces;
//! the canonical admit decision lives here, projected by LocalWorkloadSigner.

use cynic_protocol::{AgentWorkloadEnvelope, ExecutionAuthorization};

/// Admission verdict from the workload signer (reuses the 2N+1 kernel grant
/// admission shape: PolicyAdmissionReport.decision + ExecutionAuthorization).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadAdmission {
    /// Whether the workload may dispatch onto the federation runtime.
    pub admitted: bool,
    /// Derived from the admitting authority (node-owned scheduler proof).
    pub authorization: Option<ExecutionAuthorization>,
    /// Residual unknowns after the admission decision (AGENS: "Unknown is data").
    pub residual_unknowns: Vec<String>,
}

/// Phase 3.1 contract: decide whether an AgentWorkloadEnvelope may execute
/// federation mutation. Implementations are swappable adapters; the trait is
/// the durable surface (AGENS §7 adapters disposable / ports durable).
pub trait WorkloadSignerPort: Send + Sync {
    /// Decide whether an AgentWorkloadEnvelope may execute federation mutation.
    /// Returns Ok(WorkloadAdmission) on policy decision, Err on signaller fault.
    fn admit_workload(&self, envelope: &AgentWorkloadEnvelope)
    -> Result<WorkloadAdmission, String>;
}

/// Phase 3.1 null signer — federation mutation refused with explicit blocker
/// until a real workload signer (Phase 3.1) is injected. Mirrors the original
/// "runtime_federation_workload_signer_unavailable" refusal posture.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullWorkloadSigner;

impl WorkloadSignerPort for NullWorkloadSigner {
    fn admit_workload(
        &self,
        _envelope: &AgentWorkloadEnvelope,
    ) -> Result<WorkloadAdmission, String> {
        Ok(WorkloadAdmission {
            admitted: false,
            authorization: None,
            residual_unknowns: vec!["null_workload_signer".to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::workload_signer_port::{WorkloadAdmission, WorkloadSignerPort};
    use cynic_protocol::AgentWorkloadEnvelope;

    /// Minimal envelope stub for port-level admit shape coverage.
    fn stub_envelope() -> AgentWorkloadEnvelope {
        use cynic_protocol::{
            AGENT_WORKLOAD_ENVELOPE_SCHEMA, AgentBoundedAction, AgentDelegationEnvelope,
            AgentEvidenceRef, POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport,
            PolicyAuthority, PolicyDecision, PolicyScope, RequiredProof,
        };
        AgentWorkloadEnvelope {
            schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
            workload_id: "fed-1".to_string(),
            idempotency_key: "fed-1".to_string(),
            plan_digest: "sha256:plan".to_string(),
            input: None,
            action: AgentBoundedAction {
                argv: vec!["cynic".to_string()],
                working_directory: None,
                input_digest: Some("sha256:input".to_string()),
                mutates: true,
                requires_privilege: false,
                stop_condition: "done".to_string(),
                rollback_intent: "noop".to_string(),
            },
            delegation: AgentDelegationEnvelope {
                schema: "cynic.agent_delegation_envelope.v1".to_string(),
                delegation_id: "delegation-1".to_string(),
                node_id: "builder-1".to_string(),
                agent_id: "runtime_publisher@dom".to_string(),
                parent_agent_id: None,
                depth: 0,
                objective: "dom".to_string(),
                authority_snapshot: AgentEvidenceRef {
                    kind: "auth".to_string(),
                    subject: "builder-1".to_string(),
                    digest: "sha256:auth".to_string(),
                    lineage: None,
                },
                context_refs: vec![],
                capabilities: vec![cynic_protocol::AgentCapabilityGrant {
                    id: "federation_dispatch".to_string(),
                    mutates: true,
                }],
                budget: cynic_protocol::AgentBudget {
                    context_tokens: 128,
                    actions: 1,
                    children: 0,
                    wall_time_ms: 1_000,
                },
                expected_proofs: vec![],
            },
            parent_delegation: None,
            policy_admission: PolicyAdmissionReport::new(
                cynic_protocol::PolicySetRef {
                    id: "runtime_publisher".to_string(),
                    version: "v1".to_string(),
                    digest: "sha256:ps".to_string(),
                },
                cynic_protocol::PolicyIntentRef {
                    id: "peer_stimulus".to_string(),
                    kind: "federation_dispatch".to_string(),
                    actor: "builder-1".to_string(),
                    subject: "target".to_string(),
                    digest: "sha256:intent".to_string(),
                    plan_digest: "sha256:plan".to_string(),
                },
                cynic_protocol::MutationContract {
                    kind: "federation_mutation".to_string(),
                    scope: PolicyScope {
                        domain: "dom".to_string(),
                        target: "target".to_string(),
                    },
                    mutates: true,
                    requires_privilege: false,
                    requires_explicit_apply: true,
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    preconditions: vec![],
                    rollback_intent: "noop".to_string(),
                    expected_proofs: vec![],
                },
                PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: vec![RequiredProof {
                        gate: "runtime_publisher_admission".to_string(),
                        subject: "target".to_string(),
                    }],
                },
            ),
            execution_authorization: ExecutionAuthorization {
                schema: "cynic.execution_authorization.v1".to_string(),
                actor: "builder-1".to_string(),
                authority: PolicyAuthority::RuntimeAuthority,
                explicit_apply: true,
                satisfied_preconditions: vec![],
            },
        }
    }

    #[test]
    fn null_signer_refuses_federation_mutation() {
        let admission = NullWorkloadSigner
            .admit_workload(&stub_envelope())
            .expect("signer must decide");
        assert!(!admission.admitted);
        assert_eq!(
            admission.residual_unknowns,
            vec!["null_workload_signer".to_string()]
        );
    }
}
