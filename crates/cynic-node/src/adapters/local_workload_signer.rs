//! 2N+2 federation workload signer adapter — binds the kernel/node-signed
//! 2N+1 runtime_publisher grant (PolicyAdmissionReport) into the websocket
//! federation dispatch gate (Phase 3.1 workload signer port).
//!
//! Admits a federation AgentWorkloadEnvelope iff the node's own
//! runtime_publisher capability (infra/runtime_proof.v1) is granted AND the
//! workload's policy_admission is admitted (recursion admission).
//! AGENS.md §4: RuntimeMachineIdentity — the signer reuses the admission
//! report produced by the 2N+1 bootstrap grant, no HumanAdmin dependency.
//! AGENS.md §7: adapters disposable — this LocalWorkloadSigner is swappable
//! for a kernel-backed signer once Phase 3.1 (node-owned scheduler) is live.

use cynic_protocol::{AgentWorkloadEnvelope, PolicyAuthority};

use crate::ports::workload_signer_port::{WorkloadAdmission, WorkloadSignerPort};

#[derive(Debug, Clone)]
pub struct LocalWorkloadSigner {
    /// Node's own runtime_publisher admission proof (2N+1 grant presence).
    granted: bool,
    node_identity: String,
    authority: PolicyAuthority,
}

impl LocalWorkloadSigner {
    pub fn new(granted: bool, node_identity: String) -> Self {
        Self {
            granted,
            node_identity,
            authority: PolicyAuthority::RuntimeAuthority,
        }
    }
}

impl Default for LocalWorkloadSigner {
    fn default() -> Self {
        Self::new(false, "local-node".to_string())
    }
}

impl WorkloadSignerPort for LocalWorkloadSigner {
    fn admit_workload(
        &self,
        envelope: &AgentWorkloadEnvelope,
    ) -> Result<WorkloadAdmission, String> {
        // 2N+2 federation gate: admit iff node's runtime_publisher grant exists
        // and the workload's policy_admission is admitted (recursion admission).
        if !self.granted {
            return Ok(WorkloadAdmission {
                admitted: false,
                authorization: None,
                residual_unknowns: vec!["runtime_publisher_grant_absent".to_string()],
            });
        }
        if !envelope.policy_admission.admitted() {
            return Ok(WorkloadAdmission {
                admitted: false,
                authorization: None,
                residual_unknowns: vec!["workload_policy_admission_not_granted".to_string()],
            });
        }
        let authorization = cynic_protocol::ExecutionAuthorization {
            schema: "cynic.execution_authorization.v1".to_string(),
            actor: self.node_identity.clone(),
            authority: self.authority.clone(),
            explicit_apply: true,
            satisfied_preconditions: vec!["runtime_publisher_admission".to_string()],
        };
        Ok(WorkloadAdmission {
            admitted: true,
            authorization: Some(authorization),
            residual_unknowns: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admitted_envelope() -> AgentWorkloadEnvelope {
        use cynic_protocol::{
            AGENT_WORKLOAD_ENVELOPE_SCHEMA, AgentBoundedAction, AgentDelegationEnvelope,
            AgentEvidenceRef, PolicyAdmissionReport, PolicyDecision, PolicyScope, RequiredProof,
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
                    expected_proofs: vec![RequiredProof {
                        gate: "runtime_publisher_admission".to_string(),
                        subject: "target".to_string(),
                    }],
                },
                PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: vec![RequiredProof {
                        gate: "runtime_publisher_admission".to_string(),
                        subject: "target".to_string(),
                    }],
                },
            ),
            execution_authorization: cynic_protocol::ExecutionAuthorization {
                schema: "cynic.execution_authorization.v1".to_string(),
                actor: "builder-1".to_string(),
                authority: PolicyAuthority::RuntimeAuthority,
                explicit_apply: true,
                satisfied_preconditions: vec![],
            },
        }
    }

    #[test]
    fn local_signer_admits_when_grant_present_and_policy_admitted() {
        let signer = LocalWorkloadSigner::new(true, "builder-1".to_string());
        let admit = signer
            .admit_workload(&admitted_envelope())
            .expect("signer must decide");
        assert!(admit.admitted);
        let auth = admit.authorization.expect("admission must produce authorization");
        assert_eq!(auth.actor, "builder-1");
        assert!(auth.explicit_apply);
        assert!(auth
            .satisfied_preconditions
            .contains(&"runtime_publisher_admission".to_string()));
    }

    #[test]
    fn local_signer_denies_when_runtime_publisher_grant_absent() {
        let signer = LocalWorkloadSigner::new(false, "builder-1".to_string());
        let admit = signer
            .admit_workload(&admitted_envelope())
            .expect("signer must decide");
        assert!(!admit.admitted);
        assert!(admit
            .residual_unknowns
            .contains(&"runtime_publisher_grant_absent".to_string()));
        assert!(admit.authorization.is_none());
    }
}
