use cynic_protocol::{
    NodeIdentityDesiredState, SecurityEvidenceState, WORKLOAD_IDENTITY_OBSERVATION_SCHEMA,
    WorkloadIdentityDesiredState, WorkloadIdentityObservation,
};

use crate::adapters::local_node_identity::LocalNodeIdentity;
use crate::ports::node_identity_port::NodeIdentityPort;
use crate::ports::workload_identity_port::WorkloadIdentityPort;

/// Local adapter for the first bootstrap identity. It deliberately does not
/// claim issuer, audience, expiry, or generation proof: those facts belong to
/// the enrollment/credential adapter and remain explicit unknowns until read
/// back from it.
#[derive(Debug, Clone)]
pub struct LocalWorkloadIdentityAdapter {
    node_id: String,
    signing_key_path: String,
}

impl LocalWorkloadIdentityAdapter {
    pub fn new(node_id: impl Into<String>, signing_key_path: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            signing_key_path: signing_key_path.into(),
        }
    }
}

impl WorkloadIdentityPort for LocalWorkloadIdentityAdapter {
    fn observe(
        &self,
        desired: &WorkloadIdentityDesiredState,
        _now_unix_seconds: u64,
    ) -> WorkloadIdentityObservation {
        let node_desired = NodeIdentityDesiredState {
            schema: cynic_protocol::NODE_IDENTITY_DESIRED_STATE_SCHEMA.to_string(),
            node_id: self.node_id.clone(),
            signing_key_path: self.signing_key_path.clone(),
        };
        let observed = LocalNodeIdentity.observe(&node_desired);
        let (state, subject) = match observed.key_state {
            SecurityEvidenceState::Proven => (
                cynic_protocol::WorkloadIdentityState::Present,
                Some(desired.subject.clone()),
            ),
            SecurityEvidenceState::Absent => (cynic_protocol::WorkloadIdentityState::Absent, None),
            _ => (cynic_protocol::WorkloadIdentityState::Unknown, None),
        };
        let mut unknowns = observed.unknowns;
        unknowns.extend([
            "workload_identity_issuer_not_provided_by_local_key_adapter".to_string(),
            "workload_identity_audience_not_provided_by_local_key_adapter".to_string(),
            "workload_identity_expiry_not_provided_by_local_key_adapter".to_string(),
            "workload_identity_generation_not_provided_by_local_key_adapter".to_string(),
        ]);
        WorkloadIdentityObservation {
            schema: WORKLOAD_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            state,
            subject,
            issuer: None,
            audience: None,
            expires_at_unix_seconds: None,
            generation: None,
            evidence_refs: observed.evidence_refs,
            unknowns,
        }
    }

    fn enroll(&self, _desired: &WorkloadIdentityDesiredState) -> Result<(), String> {
        Err("local_workload_identity_requires_enrollment_authority".to_string())
    }

    fn renew(&self, _desired: &WorkloadIdentityDesiredState) -> Result<(), String> {
        Err("local_workload_identity_requires_credential_adapter".to_string())
    }
}
