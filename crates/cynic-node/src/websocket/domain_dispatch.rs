use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::PendingVerdict;
use crate::ports::workload_signer_port::WorkloadSignerPort;

/// Build the federation AgentWorkloadEnvelope for a peer stimulus.
/// Phase 3.1: the gate admits/denies on the *signer policy decision*; the exact
/// stimulus -> envelope binding (Phase 3.2 wire mapping) is layered on top.
fn stimulus_envelope(domain: &str, stimulus_id: &str) -> cynic_protocol::AgentWorkloadEnvelope {
    use cynic_protocol::{
        AGENT_WORKLOAD_ENVELOPE_SCHEMA, AgentBoundedAction, AgentDelegationEnvelope,
        AgentEvidenceRef, AgentWorkloadEnvelope, POLICY_ADMISSION_REPORT_SCHEMA,
        PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyScope, RequiredProof,
    };
    AgentWorkloadEnvelope {
        schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
        workload_id: stimulus_id.to_string(),
        idempotency_key: stimulus_id.to_string(),
        plan_digest: format!("sha256:stimulus-{}", stimulus_id),
        input: None,
        action: AgentBoundedAction {
            argv: vec![
                "cynic".to_string(),
                "federation".to_string(),
                "dispatch".to_string(),
            ],
            working_directory: None,
            input_digest: Some(format!("sha256:{}", stimulus_id)),
            mutates: true,
            requires_privilege: false,
            stop_condition: "stimulus_verdict_delivered".to_string(),
            rollback_intent: "revert peer runtime_publisher_admission state on verdict failure"
                .to_string(),
        },
        delegation: AgentDelegationEnvelope {
            schema: "cynic.agent_delegation_envelope.v1".to_string(),
            delegation_id: format!("federation-{}", stimulus_id),
            node_id: "local-node".to_string(),
            agent_id: format!("runtime_publisher@{domain}"),
            parent_agent_id: None,
            depth: 0,
            objective: domain.to_string(),
            authority_snapshot: AgentEvidenceRef {
                kind: "peer_stimulus".to_string(),
                subject: stimulus_id.to_string(),
                digest: format!("sha256:{}", stimulus_id.len()),
                lineage: None,
            },
            context_refs: vec![AgentEvidenceRef {
                kind: "stimulus_id".to_string(),
                subject: stimulus_id.to_string(),
                digest: "sha256:stimulus".to_string(),
                lineage: None,
            }],
            capabilities: vec![cynic_protocol::AgentCapabilityGrant {
                id: "federation_dispatch".to_string(),
                mutates: true,
            }],
            budget: cynic_protocol::AgentBudget {
                context_tokens: 256,
                actions: 1,
                children: 0,
                wall_time_ms: 5_000,
            },
            expected_proofs: vec!["runtime_publisher_admission".to_string()],
        },
        parent_delegation: None,
        policy_admission: PolicyAdmissionReport {
            schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
            policy_set: cynic_protocol::PolicySetRef {
                id: "runtime_publisher".to_string(),
                version: "v1".to_string(),
                digest: format!("sha256:{}", "0".repeat(64)),
            },
            intent: cynic_protocol::PolicyIntentRef {
                id: "peer_stimulus".to_string(),
                kind: "federation_dispatch".to_string(),
                actor: "local-node".to_string(),
                subject: stimulus_id.to_string(),
                digest: format!("sha256:{}", "0".repeat(64)),
                plan_digest: format!("sha256:{}", "0".repeat(64)),
            },
            mutation: cynic_protocol::MutationContract {
                kind: "federation_mutation".to_string(),
                scope: PolicyScope {
                    domain: domain.to_string(),
                    target: stimulus_id.to_string(),
                },
                mutates: true,
                requires_privilege: false,
                requires_explicit_apply: true,
                required_authority: PolicyAuthority::RuntimePrincipal,
                preconditions: vec![],
                rollback_intent: "none".to_string(),
                expected_proofs: vec![RequiredProof {
                    gate: "runtime_publisher_admission".to_string(),
                    subject: stimulus_id.to_string(),
                }],
            },
            decision: PolicyDecision::Allow {
                required_authority: PolicyAuthority::RuntimePrincipal,
                required_proofs: vec![RequiredProof {
                    gate: "runtime_publisher_admission".to_string(),
                    subject: stimulus_id.to_string(),
                }],
            },
        },
        execution_authorization: cynic_protocol::ExecutionAuthorization {
            schema: "cynic.execution_authorization.v1".to_string(),
            actor: "local-node".to_string(),
            authority: PolicyAuthority::RuntimeAuthority,
            explicit_apply: true,
            satisfied_preconditions: vec!["runtime_publisher_admission".to_string()],
        },
    }
}

/// Phase 3.1: native federation mutation dispatches iff the injected workload
/// signer admits the AgentWorkloadEnvelope for this stimulus.
/// Replaces the "runtime_federation_workload_signer_unavailable" blocker.
pub(super) fn spawn_native_handler(
    domain: &str,
    stimulus_id: String,
    _content: serde_json::Value,
    _verdicts_queue: Arc<Mutex<VecDeque<PendingVerdict>>>,
    signer: &dyn WorkloadSignerPort,
) -> bool {
    let envelope = stimulus_envelope(domain, &stimulus_id);
    match signer.admit_workload(&envelope) {
        Ok(admission) if admission.admitted => {
            tracing::debug!(
                domain = domain,
                workload_id = %stimulus_id,
                "federation mutation admitted by workload signer"
            );
            true
        }
        Ok(_) | Err(_) => {
            tracing::warn!(
                domain = domain,
                workload_id = %stimulus_id,
                blocker = "workload_signer_denied_federation_mutation",
                "native federation mutation refused"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AllowSigner;
    impl WorkloadSignerPort for AllowSigner {
        fn admit_workload(
            &self,
            _envelope: &cynic_protocol::AgentWorkloadEnvelope,
        ) -> Result<WorkloadAdmission, String> {
            Ok(WorkloadAdmission {
                admitted: true,
                authorization: None,
                residual_unknowns: vec![],
            })
        }
    }

    struct DenySigner;
    impl WorkloadSignerPort for DenySigner {
        fn admit_workload(
            &self,
            _envelope: &cynic_protocol::AgentWorkloadEnvelope,
        ) -> Result<WorkloadAdmission, String> {
            Ok(WorkloadAdmission {
                admitted: false,
                authorization: None,
                residual_unknowns: vec!["signer_policy_violation".to_string()],
            })
        }
    }

    fn empty_queue() -> Arc<Mutex<VecDeque<PendingVerdict>>> {
        Arc::new(Mutex::new(VecDeque::new()))
    }

    #[test]
    fn federation_mutation_admitted_when_signer_admits() {
        let admitted = spawn_native_handler(
            "runtime_publisher",
            "stimulus-1".to_string(),
            serde_json::Value::Null,
            empty_queue(),
            &AllowSigner,
        );
        assert!(
            admitted,
            "signer admission must dispatch federation stimulus"
        );
    }

    #[test]
    fn federation_mutation_refused_when_signer_denies() {
        let refused = spawn_native_handler(
            "runtime_publisher",
            "stimulus-2".to_string(),
            serde_json::Value::Null,
            empty_queue(),
            &DenySigner,
        );
        assert!(!refused, "signer denial must block federation stimulus");
    }
}
