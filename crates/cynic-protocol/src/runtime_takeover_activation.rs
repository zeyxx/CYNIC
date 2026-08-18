//! Supervisor-neutral activation contract for a proved runtime takeover.
//!
//! The kernel may produce an admissible takeover decision, but a host/node
//! supervisor owns the actual process transition. These types make that
//! boundary explicit and require read-back of identity and fencing.

use serde::{Deserialize, Serialize};

pub const RUNTIME_TAKEOVER_ACTIVATION_SCHEMA: &str = "cynic.runtime_takeover.activation.v1";
pub const RUNTIME_TAKEOVER_WATCH_SCHEMA: &str = "cynic.runtime_takeover.watch.v1";
pub const RUNTIME_TAKEOVER_ACTIVATION_ACK_SCHEMA: &str = "cynic.runtime_takeover.activation_ack.v1";
pub const RUNTIME_TAKEOVER_ADMISSION_PROOF_SCHEMA: &str =
    "cynic.runtime_takeover.admission_proof.v1";

/// Durable kernel-owned admission that a node must read back before applying
/// a host takeover. A spool record may reference this proof but never replaces
/// it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverAdmissionProof {
    pub schema: String,
    pub proof_ref: String,
    pub node_id: String,
    pub observed_at_unix_ms: u128,
    pub observation: crate::RuntimeTakeoverObservation,
    pub decision: RuntimeTakeoverWatchDecision,
}

/// Durable hand-off from the kernel admission observer to the node-owned
/// supervisor. The record is evidence only: the node must still reconcile the
/// observation and perform host-side read-back before it can activate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverWatchRecord {
    pub schema: String,
    pub observed_at_unix_ms: u128,
    pub observation: crate::RuntimeTakeoverObservation,
    pub decision: RuntimeTakeoverWatchDecision,
    pub activation_request: Option<RuntimeTakeoverActivationRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverWatchDecision {
    pub status: String,
    pub promote: bool,
    pub fencing_token: u64,
}

/// Durable supervisor read-back associated with one kernel admission proof.
/// It intentionally contains only portable activation evidence, not a
/// host-adapter-specific execution report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverActivationAck {
    pub schema: String,
    pub admission_proof_ref: String,
    pub node_id: String,
    pub fencing_token: u64,
    pub status: String,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverActivationRequest {
    pub schema: String,
    pub node_id: String,
    pub fencing_token: u64,
    pub prior_fencing_token: u64,
    pub admission_proof_ref: String,
    pub standby_unit: String,
    pub active_unit: String,
    pub health_endpoint: String,
    pub identity_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverActivationObservation {
    pub schema: String,
    pub node_id: String,
    pub active_unit: String,
    pub process_active: bool,
    pub observed_fencing_token: u64,
    pub identity_verified: bool,
    pub proof_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverActivationDecision {
    pub schema: String,
    pub apply_allowed: bool,
    pub blockers: Vec<String>,
}

pub fn reconcile_takeover_activation(
    request: &RuntimeTakeoverActivationRequest,
    observed: &RuntimeTakeoverActivationObservation,
) -> RuntimeTakeoverActivationDecision {
    let mut blockers = Vec::new();
    if request.schema != RUNTIME_TAKEOVER_ACTIVATION_SCHEMA
        || observed.schema != RUNTIME_TAKEOVER_ACTIVATION_SCHEMA
    {
        blockers.push("takeover_activation_schema_invalid".to_string());
    }
    if request.node_id.trim().is_empty() || request.node_id != observed.node_id {
        blockers.push("takeover_activation_identity_mismatch".to_string());
    }
    if request.active_unit.trim().is_empty() || request.active_unit != observed.active_unit {
        blockers.push("takeover_activation_unit_read_back_mismatch".to_string());
    }
    if request.fencing_token == 0 || request.fencing_token <= request.prior_fencing_token {
        blockers.push("takeover_activation_fencing_invalid".to_string());
    }
    if observed.observed_fencing_token != request.fencing_token {
        blockers.push("takeover_activation_fencing_read_back_mismatch".to_string());
    }
    if !observed.process_active {
        blockers.push("takeover_activation_process_inactive".to_string());
    }
    if !observed.identity_verified {
        blockers.push("takeover_activation_identity_unproved".to_string());
    }
    if request.admission_proof_ref.trim().is_empty() {
        blockers.push("takeover_activation_admission_proof_missing".to_string());
    }
    if observed.proof_refs.is_empty() {
        blockers.push("takeover_activation_read_back_proof_missing".to_string());
    }
    RuntimeTakeoverActivationDecision {
        schema: RUNTIME_TAKEOVER_ACTIVATION_SCHEMA.to_string(),
        apply_allowed: blockers.is_empty(),
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RuntimeTakeoverActivationRequest {
        RuntimeTakeoverActivationRequest {
            schema: RUNTIME_TAKEOVER_ACTIVATION_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            fencing_token: 4,
            prior_fencing_token: 3,
            admission_proof_ref: "spool://takeover-watch/4".to_string(),
            standby_unit: "cynic-kernel-standby.service".to_string(),
            active_unit: "cynic-kernel.service".to_string(),
            health_endpoint: "http://127.0.0.1:3030/health".to_string(),
            identity_endpoint: "http://127.0.0.1:3030/identity".to_string(),
        }
    }

    #[test]
    fn activation_requires_identity_fence_and_read_back() {
        let request = request();
        let observed = RuntimeTakeoverActivationObservation {
            schema: RUNTIME_TAKEOVER_ACTIVATION_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            active_unit: "cynic-kernel.service".to_string(),
            process_active: true,
            observed_fencing_token: 4,
            identity_verified: true,
            proof_refs: vec!["systemd://cynic-kernel.service/active".to_string()],
        };
        assert!(reconcile_takeover_activation(&request, &observed).apply_allowed);

        let mut stale = observed;
        stale.observed_fencing_token = 3;
        assert!(!reconcile_takeover_activation(&request, &stale).apply_allowed);
    }
}
