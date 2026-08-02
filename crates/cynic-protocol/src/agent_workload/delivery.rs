use serde::{Deserialize, Serialize};

use super::{AgentWorkloadEnvelope, admit_agent_workload};

pub const AGENT_WORKLOAD_DELIVERY_SCHEMA: &str = "cynic.agent_workload_delivery.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkloadDeliveryStatus {
    Pending,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadDelivery {
    pub schema: String,
    pub envelope: AgentWorkloadEnvelope,
    pub target_node_id: String,
    pub submitted_by: String,
    pub submitted_at_ms: u64,
    pub status: AgentWorkloadDeliveryStatus,
    #[serde(default)]
    pub accepted_at_ms: Option<u64>,
    #[serde(default)]
    pub accepted_record_digest: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

pub fn validate_agent_workload_delivery(delivery: &AgentWorkloadDelivery) -> Vec<String> {
    let mut violations = Vec::new();
    if delivery.schema != AGENT_WORKLOAD_DELIVERY_SCHEMA {
        violations.push("agent_workload_delivery_schema_mismatch".to_string());
    }
    if delivery.target_node_id != delivery.envelope.delegation.node_id
        || delivery.submitted_by != delivery.envelope.delegation.agent_id
    {
        violations.push("agent_workload_delivery_identity_mismatch".to_string());
    }
    if delivery.submitted_at_ms == 0 || delivery.evidence_refs.is_empty() {
        violations.push("agent_workload_delivery_evidence_missing".to_string());
    }
    if !admit_agent_workload(&delivery.envelope).admitted() {
        violations.push("agent_workload_delivery_envelope_not_admitted".to_string());
    }
    match delivery.status {
        AgentWorkloadDeliveryStatus::Pending
            if delivery.accepted_at_ms.is_some() || delivery.accepted_record_digest.is_some() =>
        {
            violations.push("pending_agent_workload_delivery_has_acceptance".to_string());
        }
        AgentWorkloadDeliveryStatus::Accepted => {
            if delivery
                .accepted_at_ms
                .is_none_or(|value| value < delivery.submitted_at_ms)
                || delivery
                    .accepted_record_digest
                    .as_deref()
                    .is_none_or(|digest| !valid_sha256(digest))
            {
                violations.push("agent_workload_delivery_acceptance_invalid".to_string());
            }
        }
        AgentWorkloadDeliveryStatus::Pending => {}
    }
    violations.sort();
    violations.dedup();
    violations
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BUILD_AGENT_REQUEST_SCHEMA, BuildAgentRequest, compose_build_agent_workload};

    fn delivery() -> AgentWorkloadDelivery {
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "delivery-test".to_string(),
            requested_by: "agent-test".to_string(),
            source_commit: "a".repeat(40),
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["canonical_gate_report".to_string()],
            evidence_refs: vec!["git:commit:test".to_string()],
        };
        AgentWorkloadDelivery {
            schema: AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
            envelope: compose_build_agent_workload(
                &request,
                "cynic-builder",
                "/var/lib/cynic/source/workspaces",
            )
            .unwrap(),
            target_node_id: "cynic-builder".to_string(),
            submitted_by: "agent-test".to_string(),
            submitted_at_ms: 1,
            status: AgentWorkloadDeliveryStatus::Pending,
            accepted_at_ms: None,
            accepted_record_digest: None,
            evidence_refs: vec!["git:commit:test".to_string()],
        }
    }

    #[test]
    fn pending_delivery_binds_submitter_target_and_admitted_envelope() {
        assert!(validate_agent_workload_delivery(&delivery()).is_empty());
    }

    #[test]
    fn accepted_delivery_requires_content_addressed_durable_read_back() {
        let mut delivery = delivery();
        delivery.status = AgentWorkloadDeliveryStatus::Accepted;
        delivery.accepted_at_ms = Some(2);
        delivery.accepted_record_digest = Some("not-a-digest".to_string());

        assert_eq!(
            validate_agent_workload_delivery(&delivery),
            vec!["agent_workload_delivery_acceptance_invalid"]
        );
    }
}
