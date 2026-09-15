use serde::{Deserialize, Serialize};

use super::{
    AgentWorkloadEnvelope, DurableAgentWorkloadRecord, admit_agent_workload,
    validate_durable_agent_workload_record,
};
use cynic_protocol::{
    AgentCompletionEnvelope, EvidenceEnvelope, admit_completion,
    build_evidence_envelope_with_lineage,
};

pub const AGENT_WORKLOAD_DELIVERY_SCHEMA: &str = "cynic.agent_workload_delivery.v1";
pub const AGENT_WORKLOAD_DELIVERY_LEASE_SCHEMA: &str = "cynic.agent_workload_delivery_lease.v1";
pub const AGENT_WORKLOAD_DELIVERY_LEASE_MAX_TTL_MS: u64 = 15 * 60 * 1_000;
pub const AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA: &str =
    "cynic.agent_workload_completion_publication.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadDeliveryLease {
    pub schema: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub consumer_instance_id: String,
    pub accepted_at_ms: u64,
    pub expires_at_ms: u64,
    pub record_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadDeliveryAcceptance {
    pub consumer_instance_id: String,
    pub record_digest: String,
    pub requested_at_ms: u64,
    pub lease_ttl_ms: u64,
    #[serde(default)]
    pub prior_lease_id: Option<String>,
    #[serde(default)]
    pub prior_fencing_token: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkloadDeliveryStatus {
    Pending,
    Accepted,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadCompletionPublication {
    pub schema: String,
    pub node_id: String,
    pub workload_id: String,
    pub published_at_ms: u64,
    pub delivery_lease: AgentWorkloadDeliveryLease,
    pub durable_record: DurableAgentWorkloadRecord,
    #[serde(default)]
    pub evidence: Vec<EvidenceEnvelope>,
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
    pub acceptance_lease: Option<AgentWorkloadDeliveryLease>,
    #[serde(default)]
    pub terminal_at_ms: Option<u64>,
    #[serde(default)]
    pub terminal_record_digest: Option<String>,
    #[serde(default)]
    pub terminal_record: Option<DurableAgentWorkloadRecord>,
    #[serde(default)]
    pub completion: Option<AgentCompletionEnvelope>,
    #[serde(default)]
    pub completion_evidence: Vec<EvidenceEnvelope>,
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
            if has_acceptance(delivery) || has_completion(delivery) =>
        {
            violations.push("pending_agent_workload_delivery_has_acceptance".to_string());
        }
        AgentWorkloadDeliveryStatus::Accepted => {
            if !valid_acceptance(delivery) {
                violations.push("agent_workload_delivery_acceptance_invalid".to_string());
            }
            if has_completion(delivery) {
                violations.push("accepted_agent_workload_delivery_has_completion".to_string());
            }
        }
        AgentWorkloadDeliveryStatus::Terminal => {
            if !valid_acceptance(delivery) {
                violations.push("agent_workload_delivery_acceptance_invalid".to_string());
            }
            if delivery.terminal_at_ms.is_none_or(|value| {
                delivery
                    .accepted_at_ms
                    .is_none_or(|accepted| value < accepted)
            }) || delivery
                .terminal_record_digest
                .as_deref()
                .is_none_or(|digest| !valid_sha256(digest))
            {
                violations.push("agent_workload_delivery_terminal_read_back_invalid".to_string());
            }
            match &delivery.terminal_record {
                Some(record)
                    if validate_durable_agent_workload_record(record).is_empty()
                        && delivery.terminal_record_digest.as_deref()
                            == Some(record.record_digest.as_str())
                        && record.record.envelope == delivery.envelope
                        && record.record.state.terminal()
                        && record.record.completion == delivery.completion => {}
                _ => violations.push("agent_workload_delivery_terminal_record_invalid".to_string()),
            }
            match &delivery.completion {
                Some(completion)
                    if admit_completion(&delivery.envelope.delegation, completion).admitted() => {}
                _ => violations.push("agent_workload_delivery_completion_invalid".to_string()),
            }
            validate_completion_evidence(
                delivery.completion.as_ref(),
                &delivery.completion_evidence,
                &mut violations,
            );
        }
        AgentWorkloadDeliveryStatus::Pending => {}
    }
    violations.sort();
    violations.dedup();
    violations
}

pub fn validate_agent_workload_completion_publication(
    delivery: &AgentWorkloadDelivery,
    publication: &AgentWorkloadCompletionPublication,
) -> Vec<String> {
    let mut violations = validate_agent_workload_delivery(delivery);
    if publication.schema != AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA {
        violations.push("agent_workload_completion_publication_schema_mismatch".to_string());
    }
    if delivery.status != AgentWorkloadDeliveryStatus::Accepted {
        violations.push("agent_workload_completion_delivery_not_accepted".to_string());
    }
    if delivery.acceptance_lease.as_ref() != Some(&publication.delivery_lease) {
        violations.push("agent_workload_completion_delivery_lease_mismatch".to_string());
    }
    if publication.delivery_lease.record_digest != publication.durable_record.record_digest {
        violations.push("agent_workload_completion_delivery_record_mismatch".to_string());
    }
    if publication.node_id != delivery.target_node_id
        || publication.workload_id != delivery.envelope.workload_id
        || publication.durable_record.record.envelope != delivery.envelope
    {
        violations.push("agent_workload_completion_identity_mismatch".to_string());
    }
    if publication.published_at_ms == 0
        || delivery
            .accepted_at_ms
            .is_some_and(|accepted| publication.published_at_ms < accepted)
    {
        violations.push("agent_workload_completion_publication_time_invalid".to_string());
    }
    if publication.published_at_ms > publication.delivery_lease.expires_at_ms {
        violations.push("agent_workload_completion_delivery_lease_expired".to_string());
    }
    violations.extend(validate_durable_agent_workload_record(
        &publication.durable_record,
    ));
    if !publication.durable_record.record.state.terminal() {
        violations.push("agent_workload_completion_record_not_terminal".to_string());
    }
    let completion = publication.durable_record.record.completion.as_ref();
    match completion {
        Some(completion)
            if admit_completion(&delivery.envelope.delegation, completion).admitted() => {}
        _ => violations.push("agent_workload_completion_envelope_invalid".to_string()),
    }
    validate_completion_evidence(completion, &publication.evidence, &mut violations);
    violations.sort();
    violations.dedup();
    violations
}

pub fn agent_workload_delivery_available(delivery: &AgentWorkloadDelivery, now_ms: u64) -> bool {
    match delivery.status {
        AgentWorkloadDeliveryStatus::Pending => true,
        AgentWorkloadDeliveryStatus::Accepted => delivery
            .acceptance_lease
            .as_ref()
            .is_none_or(|lease| lease.expires_at_ms <= now_ms),
        AgentWorkloadDeliveryStatus::Terminal => false,
    }
}

pub fn validate_agent_workload_delivery_lease(lease: &AgentWorkloadDeliveryLease) -> Vec<String> {
    let mut violations = Vec::new();
    if lease.schema != AGENT_WORKLOAD_DELIVERY_LEASE_SCHEMA
        || !valid_sha256(&lease.lease_id)
        || lease.fencing_token == 0
        || lease.consumer_instance_id.trim().is_empty()
        || lease.accepted_at_ms == 0
        || lease.expires_at_ms <= lease.accepted_at_ms
        || !valid_sha256(&lease.record_digest)
    {
        violations.push("agent_workload_delivery_lease_invalid".to_string());
    }
    violations
}

pub fn accept_agent_workload_delivery(
    delivery: &AgentWorkloadDelivery,
    acceptance: &AgentWorkloadDeliveryAcceptance,
) -> Result<AgentWorkloadDelivery, Vec<String>> {
    let mut violations = validate_agent_workload_delivery(delivery);
    if delivery.status == AgentWorkloadDeliveryStatus::Terminal {
        violations.push("terminal_agent_workload_delivery_cannot_be_accepted".to_string());
    }
    if acceptance.consumer_instance_id.trim().is_empty()
        || !valid_sha256(&acceptance.record_digest)
        || acceptance.requested_at_ms < delivery.submitted_at_ms
        || acceptance.lease_ttl_ms == 0
        || acceptance.lease_ttl_ms > AGENT_WORKLOAD_DELIVERY_LEASE_MAX_TTL_MS
        || acceptance
            .requested_at_ms
            .checked_add(acceptance.lease_ttl_ms)
            .is_none()
    {
        violations.push("agent_workload_delivery_acceptance_request_invalid".to_string());
    }
    let renewing = delivery.acceptance_lease.as_ref().is_some_and(|lease| {
        lease.expires_at_ms > acceptance.requested_at_ms
            && lease.consumer_instance_id == acceptance.consumer_instance_id
            && acceptance.prior_lease_id.as_deref() == Some(lease.lease_id.as_str())
            && acceptance.prior_fencing_token == Some(lease.fencing_token)
    });
    let active_conflict = delivery
        .acceptance_lease
        .as_ref()
        .is_some_and(|lease| lease.expires_at_ms > acceptance.requested_at_ms && !renewing);
    if active_conflict {
        violations.push("agent_workload_delivery_lease_active".to_string());
    }
    if !violations.is_empty() {
        violations.sort();
        violations.dedup();
        return Err(violations);
    }

    let prior = delivery.acceptance_lease.as_ref();
    let fencing_token = if renewing {
        prior.map_or(1, |lease| lease.fencing_token)
    } else {
        prior.map_or(1, |lease| lease.fencing_token.saturating_add(1))
    };
    let accepted_at_ms = if renewing {
        prior.map_or(acceptance.requested_at_ms, |lease| lease.accepted_at_ms)
    } else {
        acceptance.requested_at_ms
    };
    let lease_id = if renewing {
        prior.map_or_else(String::new, |lease| lease.lease_id.clone())
    } else {
        cynic_protocol::digest_json(&serde_json::json!({
            "workload_id": delivery.envelope.workload_id,
            "consumer_instance_id": acceptance.consumer_instance_id,
            "fencing_token": fencing_token,
            "accepted_at_ms": accepted_at_ms,
            "record_digest": acceptance.record_digest,
        }))
        .expect("workload delivery lease material is serializable")
    };
    let lease = AgentWorkloadDeliveryLease {
        schema: AGENT_WORKLOAD_DELIVERY_LEASE_SCHEMA.to_string(),
        lease_id,
        fencing_token,
        consumer_instance_id: acceptance.consumer_instance_id.clone(),
        accepted_at_ms,
        expires_at_ms: acceptance.requested_at_ms + acceptance.lease_ttl_ms,
        record_digest: acceptance.record_digest.clone(),
    };
    let mut accepted = delivery.clone();
    accepted.status = AgentWorkloadDeliveryStatus::Accepted;
    accepted.accepted_at_ms = Some(lease.accepted_at_ms);
    accepted.accepted_record_digest = Some(lease.record_digest.clone());
    accepted.acceptance_lease = Some(lease.clone());
    let evidence = format!("workload-delivery-lease:{}", lease.lease_id);
    accepted
        .evidence_refs
        .retain(|reference| !reference.starts_with("workload-delivery-lease:"));
    accepted.evidence_refs.push(evidence);
    let violations = validate_agent_workload_delivery(&accepted);
    if violations.is_empty() {
        Ok(accepted)
    } else {
        Err(violations)
    }
}

pub fn complete_agent_workload_delivery(
    delivery: &AgentWorkloadDelivery,
    publication: &AgentWorkloadCompletionPublication,
) -> Result<AgentWorkloadDelivery, Vec<String>> {
    let violations = validate_agent_workload_completion_publication(delivery, publication);
    if !violations.is_empty() {
        return Err(violations);
    }
    let mut terminal = delivery.clone();
    terminal.status = AgentWorkloadDeliveryStatus::Terminal;
    terminal.terminal_at_ms = Some(publication.published_at_ms);
    terminal.terminal_record_digest = Some(publication.durable_record.record_digest.clone());
    terminal.terminal_record = Some(publication.durable_record.clone());
    terminal.completion = publication.durable_record.record.completion.clone();
    terminal.completion_evidence = publication.evidence.clone();
    terminal.evidence_refs.push(format!(
        "durable-workload-completion:{}",
        publication.durable_record.record_digest
    ));
    let violations = validate_agent_workload_delivery(&terminal);
    if violations.is_empty() {
        Ok(terminal)
    } else {
        Err(violations)
    }
}

fn validate_completion_evidence(
    completion: Option<&AgentCompletionEnvelope>,
    evidence: &[EvidenceEnvelope],
    violations: &mut Vec<String>,
) {
    let mut expected = completion
        .into_iter()
        .flat_map(|completion| completion.evidence_refs.iter())
        .map(|reference| reference.digest.clone())
        .collect::<Vec<_>>();
    let mut observed = evidence
        .iter()
        .map(|envelope| envelope.digest.clone())
        .collect::<Vec<_>>();
    expected.sort();
    observed.sort();
    if expected != observed {
        violations.push("agent_workload_completion_evidence_set_mismatch".to_string());
    }
    for envelope in evidence {
        let rebuilt = build_evidence_envelope_with_lineage(
            &envelope.subject,
            &envelope.producer,
            envelope.observed_at_unix_ms,
            envelope.provenance.clone(),
            envelope.lineage.clone(),
            &envelope.payload,
        );
        if rebuilt
            .as_ref()
            .map_or(true, |candidate| candidate != envelope)
        {
            violations.push("agent_workload_completion_evidence_digest_invalid".to_string());
        }
    }
}

fn has_acceptance(delivery: &AgentWorkloadDelivery) -> bool {
    delivery.accepted_at_ms.is_some()
        || delivery.accepted_record_digest.is_some()
        || delivery.acceptance_lease.is_some()
}

fn has_completion(delivery: &AgentWorkloadDelivery) -> bool {
    delivery.terminal_at_ms.is_some()
        || delivery.terminal_record_digest.is_some()
        || delivery.terminal_record.is_some()
        || delivery.completion.is_some()
        || !delivery.completion_evidence.is_empty()
}

fn valid_acceptance(delivery: &AgentWorkloadDelivery) -> bool {
    let legacy_fields_valid = delivery
        .accepted_at_ms
        .is_some_and(|value| value >= delivery.submitted_at_ms)
        && delivery
            .accepted_record_digest
            .as_deref()
            .is_some_and(valid_sha256);
    legacy_fields_valid
        && delivery.acceptance_lease.as_ref().is_none_or(|lease| {
            validate_agent_workload_delivery_lease(lease).is_empty()
                && lease.accepted_at_ms >= delivery.submitted_at_ms
                && delivery.accepted_at_ms == Some(lease.accepted_at_ms)
                && delivery.accepted_record_digest.as_deref() == Some(lease.record_digest.as_str())
        })
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

/// Kernel REST envelope wrapping a persisted delivery.
///
/// Every kernel `/agent-workloads` handler returns this shape
/// (`schema`/`status`/`value`), never a bare delivery. Read-back parsers
/// must unwrap the envelope; parsing the body as a raw delivery always
/// fails and masks persisted successes as blocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelDeliveryEnvelope {
    pub schema: String,
    pub status: String,
    pub value: AgentWorkloadDelivery,
}

pub fn parse_agent_workload_observation_read_back(
    body: &serde_json::Value,
) -> Result<AgentWorkloadDelivery, Vec<String>> {
    let envelope: KernelDeliveryEnvelope = serde_json::from_value(body.clone())
        .map_err(|e| vec![format!("invalid_agent_workload_observation_read_back:{e}")])?;
    if envelope.status != "observed" {
        return Err(vec![format!(
            "agent_workload_observation_read_back_unexpected_status:{}",
            envelope.status
        )]);
    }
    Ok(envelope.value)
}

pub fn validate_agent_workload_submission_read_back(
    delivery: &AgentWorkloadDelivery,
    body: &str,
) -> Vec<String> {
    let envelope: KernelDeliveryEnvelope = match serde_json::from_str(body) {
        Ok(envelope) => envelope,
        Err(e) => return vec![format!("invalid_agent_workload_submission_read_back:{e}")],
    };
    if envelope.status != "persisted" {
        return vec![format!(
            "agent_workload_submission_read_back_unexpected_status:{}",
            envelope.status
        )];
    }
    let observed = envelope.value;
    let mut violations = validate_agent_workload_delivery(&observed);
    if observed.envelope != delivery.envelope {
        violations.push("agent_workload_submission_read_back_envelope_mismatch".to_string());
    }
    if observed.target_node_id != delivery.target_node_id {
        violations.push("agent_workload_submission_read_back_target_node_mismatch".to_string());
    }
    violations
}
