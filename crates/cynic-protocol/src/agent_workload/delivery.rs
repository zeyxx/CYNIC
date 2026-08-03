use serde::{Deserialize, Serialize};

use super::{
    AgentWorkloadEnvelope, DurableAgentWorkloadRecord, admit_agent_workload,
    validate_durable_agent_workload_record,
};
use crate::{
    AgentCompletionEnvelope, EvidenceEnvelope, admit_completion,
    build_evidence_envelope_with_lineage,
};

pub const AGENT_WORKLOAD_DELIVERY_SCHEMA: &str = "cynic.agent_workload_delivery.v1";
pub const AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA: &str =
    "cynic.agent_workload_completion_publication.v1";

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
    delivery.accepted_at_ms.is_some() || delivery.accepted_record_digest.is_some()
}

fn has_completion(delivery: &AgentWorkloadDelivery) -> bool {
    delivery.terminal_at_ms.is_some()
        || delivery.terminal_record_digest.is_some()
        || delivery.terminal_record.is_some()
        || delivery.completion.is_some()
        || !delivery.completion_evidence.is_empty()
}

fn valid_acceptance(delivery: &AgentWorkloadDelivery) -> bool {
    delivery
        .accepted_at_ms
        .is_some_and(|value| value >= delivery.submitted_at_ms)
        && delivery
            .accepted_record_digest
            .as_deref()
            .is_some_and(valid_sha256)
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AGENT_COMPLETION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, AgentBudget,
        AgentCompletionEnvelope, AgentCompletionStatus, AgentEvidenceRef, AgentWorkloadRecord,
        AgentWorkloadState, BUILD_AGENT_REQUEST_SCHEMA, BUILD_SOURCE_REFERENCE_SCHEMA,
        BuildAgentRequest, BuildSourceReference, DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA,
        DurableAgentWorkloadRecord, build_evidence_envelope, compose_build_agent_workload,
        digest_json,
    };

    fn delivery() -> AgentWorkloadDelivery {
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "delivery-test".to_string(),
            requested_by: "agent-test".to_string(),
            source_commit: "a".repeat(40),
            source: BuildSourceReference {
                schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".into(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".into(),
                revision: "a".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".into(),
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["canonical_gate_report".to_string()],
            release_publication: None,
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
            terminal_at_ms: None,
            terminal_record_digest: None,
            terminal_record: None,
            completion: None,
            completion_evidence: Vec::new(),
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

    fn publication() -> (AgentWorkloadDelivery, AgentWorkloadCompletionPublication) {
        let mut accepted = delivery();
        accepted.status = AgentWorkloadDeliveryStatus::Accepted;
        accepted.accepted_at_ms = Some(2);
        accepted.accepted_record_digest = Some(format!("sha256:{}", "1".repeat(64)));
        let evidence = build_evidence_envelope(
            "build-workload:delivery-test:canonical_gate_report",
            "cynic-node/build-scheduler:cynic-builder",
            3,
            vec!["test".to_string()],
            &serde_json::json!({"status": "passed"}),
        )
        .unwrap();
        let completion = AgentCompletionEnvelope {
            schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: accepted.envelope.delegation.delegation_id.clone(),
            node_id: accepted.target_node_id.clone(),
            agent_id: accepted.submitted_by.clone(),
            parent_agent_id: None,
            status: AgentCompletionStatus::Completed,
            evidence_refs: vec![AgentEvidenceRef {
                kind: "canonical_gate_report".to_string(),
                subject: accepted.envelope.workload_id.clone(),
                digest: evidence.digest.clone(),
                lineage: None,
            }],
            unknowns: Vec::new(),
            consumed_budget: AgentBudget {
                context_tokens: 0,
                actions: 0,
                children: 0,
                wall_time_ms: 1,
            },
        };
        let record = AgentWorkloadRecord {
            schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
            envelope: accepted.envelope.clone(),
            state: AgentWorkloadState::Completed,
            claim: None,
            last_fencing_token: 0,
            cancellation_generation: 0,
            consumed_actions: 0,
            completion: Some(completion),
        };
        let durable = DurableAgentWorkloadRecord {
            schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
            revision: 2,
            previous_record_digest: Some(format!("sha256:{}", "2".repeat(64))),
            record_digest: digest_json(&record).unwrap(),
            record,
            committed_at_ms: 3,
        };
        let publication = AgentWorkloadCompletionPublication {
            schema: AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA.to_string(),
            node_id: accepted.target_node_id.clone(),
            workload_id: accepted.envelope.workload_id.clone(),
            published_at_ms: 4,
            durable_record: durable,
            evidence: vec![evidence],
        };
        (accepted, publication)
    }

    #[test]
    fn terminal_delivery_is_derived_from_exact_durable_completion_and_evidence() {
        let (accepted, publication) = publication();

        let terminal = complete_agent_workload_delivery(&accepted, &publication).unwrap();

        assert_eq!(terminal.status, AgentWorkloadDeliveryStatus::Terminal);
        assert_eq!(
            terminal.terminal_record_digest.as_deref(),
            Some(publication.durable_record.record_digest.as_str())
        );
        assert!(validate_agent_workload_delivery(&terminal).is_empty());
    }

    #[test]
    fn altered_completion_evidence_is_rejected() {
        let (accepted, mut publication) = publication();
        publication.evidence[0].payload = serde_json::json!({"status": "forged"});

        assert_eq!(
            validate_agent_workload_completion_publication(&accepted, &publication),
            vec!["agent_workload_completion_evidence_digest_invalid"]
        );
    }
}
