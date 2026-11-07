use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::workspace::ports::AgentWorkspacePortError;
use crate::AgentWorkloadEnvelope;
use crate::{
    AgentSchedulerJournalRecord, AgentWorkloadCompletionPublication, AgentWorkloadDelivery,
    AgentWorkloadDeliveryAcceptance, AgentWorkloadDeliveryLease, AgentWorkloadRecord,
    DurableAgentWorkloadRecord,
};
use cynic_protocol::{
    ExecutionAuthorization, WorkloadIdentityDesiredState, WorkloadIdentityObservation,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWorkloadSubmissionReadBack {
    pub http_status: u16,
    pub body: String,
}

#[async_trait]
pub trait BuildWorkloadSubmissionPort: Send + Sync {
    async fn submit(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<BuildWorkloadSubmissionReadBack, String>;
}

#[async_trait]
pub trait BuildWorkloadObservationPort: Send + Sync {
    async fn observe_workload(
        &self,
        workload_id: &str,
    ) -> Result<BuildWorkloadObservationReadBack, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWorkloadObservationReadBack {
    pub http_status: u16,
    pub body: serde_json::Value,
}

pub trait AgentSchedulerJournalPort: Send + Sync {
    fn records(
        &self,
        node_id: &str,
        scheduler_epoch: u64,
        step_id: &str,
    ) -> Result<Vec<AgentSchedulerJournalRecord>, AgentWorkspacePortError>;

    fn persist(
        &self,
        record: &AgentSchedulerJournalRecord,
    ) -> Result<AgentSchedulerJournalRecord, AgentWorkspacePortError>;
}

pub trait AgentWorkloadCompletionReceiptPort: Send + Sync {
    fn contains(&self, terminal_record_digest: &str) -> Result<bool, String>;

    fn persist(&self, delivery: &AgentWorkloadDelivery) -> Result<(), String>;
}

pub trait AgentWorkloadDeliveryLeasePort: Send + Sync {
    fn read(&self, workload_id: &str) -> Result<Option<AgentWorkloadDeliveryLease>, String>;

    fn persist(&self, workload_id: &str, lease: &AgentWorkloadDeliveryLease) -> Result<(), String>;
}

pub trait AgentWorkloadDeliveryPort: Send + Sync {
    async fn pending_deliveries(&self, node_id: &str)
        -> Result<Vec<AgentWorkloadDelivery>, String>;

    async fn accept_delivery(
        &self,
        node_id: &str,
        workload_id: &str,
        acceptance: &AgentWorkloadDeliveryAcceptance,
    ) -> Result<AgentWorkloadDelivery, String>;

    async fn publish_completion(
        &self,
        publication: &AgentWorkloadCompletionPublication,
    ) -> Result<AgentWorkloadDelivery, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorkloadStoreError {
    pub code: String,
    pub detail: String,
}

impl AgentWorkloadStoreError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

pub trait AgentWorkloadStorePort: Send + Sync {
    fn read(
        &self,
        workload_id: &str,
    ) -> Result<Option<DurableAgentWorkloadRecord>, AgentWorkloadStoreError>;

    fn active_idempotency_keys(&self) -> Result<Vec<String>, AgentWorkloadStoreError>;

    fn active_records(&self) -> Result<Vec<AgentWorkloadRecord>, AgentWorkloadStoreError>;

    fn durable_records(&self) -> Result<Vec<DurableAgentWorkloadRecord>, AgentWorkloadStoreError>;

    fn compare_and_swap(
        &self,
        workload_id: &str,
        expected_revision: Option<u64>,
        next: &AgentWorkloadRecord,
        committed_at_ms: u64,
    ) -> Result<DurableAgentWorkloadRecord, AgentWorkloadStoreError>;

    fn archive_terminal(
        &self,
        workload_id: &str,
        older_than_ms: u64,
        now_ms: u64,
    ) -> Result<Option<String>, AgentWorkloadStoreError>;

    /// Append a read-only JSON evidence sidecar inside the workload directory
    /// (e.g. the settlement proof body). Never mutates revisions; fails closed
    /// on unknown workloads or unsafe file names.
    fn append_evidence(
        &self,
        workload_id: &str,
        file_name: &str,
        body: &serde_json::Value,
    ) -> Result<(), AgentWorkloadStoreError>;
}

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

/// Portable workload-identity boundary. Providers such as SPIFFE, a local
/// machine signer, or a cloud workload identity adapter implement this port;
/// callers never inspect provider credential material.
pub trait WorkloadIdentityPort: Send + Sync {
    fn observe(
        &self,
        desired: &WorkloadIdentityDesiredState,
        now_unix_seconds: u64,
    ) -> WorkloadIdentityObservation;

    fn enroll(&self, desired: &WorkloadIdentityDesiredState) -> Result<(), String>;

    fn renew(&self, desired: &WorkloadIdentityDesiredState) -> Result<(), String>;
}

#[cfg(test)]
mod workload_signer_tests {
    use super::*;
    use crate::{AgentBoundedAction, AGENT_WORKLOAD_ENVELOPE_SCHEMA};
    use cynic_protocol::{
        AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope, AgentEvidenceRef,
        ExecutionAuthorization, MutationContract, PolicyAdmissionReport, PolicyAuthority,
        PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof,
    };

    fn stub_envelope() -> AgentWorkloadEnvelope {
        AgentWorkloadEnvelope {
            schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
            workload_id: "fed-1".to_string(),
            idempotency_key: "fed-1".to_string(),
            plan_digest: "sha256:plan".to_string(),
            mandate_binding: None,
            input: None,
            source_commit: None,
            branch: None,
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
                    cpu_ms: None,
                },
                expected_proofs: vec![],
            },
            parent_delegation: None,
            policy_admission: PolicyAdmissionReport::new(
                PolicySetRef {
                    id: "runtime_publisher".to_string(),
                    version: "v1".to_string(),
                    digest: "sha256:ps".to_string(),
                },
                PolicyIntentRef {
                    id: "peer_stimulus".to_string(),
                    kind: "federation_dispatch".to_string(),
                    actor: "builder-1".to_string(),
                    subject: "target".to_string(),
                    digest: "sha256:intent".to_string(),
                    plan_digest: "sha256:plan".to_string(),
                },
                MutationContract {
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
            execution_authorization: ExecutionAuthorization {
                schema: "cynic.execution_authorization.v1".to_string(),
                actor: "builder-1".to_string(),
                authority: PolicyAuthority::RuntimeAuthority,
                explicit_apply: true,
                satisfied_preconditions: vec![],
            },
            resource_budget: None,
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
