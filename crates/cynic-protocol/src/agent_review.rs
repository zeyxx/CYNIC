//! Adapter-neutral review receipts that prevent blind forwarding of agent output.

use serde::{Deserialize, Serialize};

use crate::AgentEvidenceRef;

pub const AGENT_REVIEW_RECEIPT_SCHEMA: &str = "cynic.agent_review_receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentReviewState {
    Proposed,
    EvidenceCollected,
    ValidationPassed,
    HumanAcknowledged,
    Rejected,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReviewReceipt {
    pub schema: String,
    pub workload_id: String,
    pub proposal_digest: String,
    pub state: AgentReviewState,
    #[serde(default)]
    pub observed_input_refs: Vec<AgentEvidenceRef>,
    #[serde(default)]
    pub validation_evidence_refs: Vec<AgentEvidenceRef>,
    pub human_acknowledgement: Option<AgentEvidenceRef>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

/// True only when the receipt proves that the proposal and its inputs were
/// reviewed enough for the policy that owns the bounded action.
pub fn admit_agent_review_receipt(
    receipt: &AgentReviewReceipt,
    human_review_required: bool,
) -> Result<(), Vec<String>> {
    let mut blockers = validate_agent_review_receipt(receipt);
    if !matches!(
        receipt.state,
        AgentReviewState::ValidationPassed | AgentReviewState::HumanAcknowledged
    ) {
        blockers.push("agent_review_not_validation_complete".to_string());
    }
    if human_review_required && receipt.state != AgentReviewState::HumanAcknowledged {
        blockers.push("agent_review_human_acknowledgement_required".to_string());
    }
    if human_review_required && receipt.human_acknowledgement.is_none() {
        blockers.push("agent_review_human_acknowledgement_missing".to_string());
    }
    blockers.sort();
    blockers.dedup();
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

pub fn validate_agent_review_receipt(receipt: &AgentReviewReceipt) -> Vec<String> {
    let mut blockers = Vec::new();
    if receipt.schema != AGENT_REVIEW_RECEIPT_SCHEMA {
        blockers.push("agent_review_schema_invalid".to_string());
    }
    if receipt.workload_id.trim().is_empty() {
        blockers.push("agent_review_workload_id_missing".to_string());
    }
    if !valid_sha256_digest(&receipt.proposal_digest) {
        blockers.push("agent_review_proposal_digest_invalid".to_string());
    }
    if requires_observed_inputs(receipt.state) && receipt.observed_input_refs.is_empty() {
        blockers.push("agent_review_observed_inputs_missing".to_string());
    }
    if requires_validation_evidence(receipt.state) && receipt.validation_evidence_refs.is_empty() {
        blockers.push("agent_review_validation_evidence_missing".to_string());
    }
    if receipt.state == AgentReviewState::HumanAcknowledged
        && receipt.human_acknowledgement.is_none()
    {
        blockers.push("agent_review_human_acknowledgement_missing".to_string());
    }
    for reference in receipt
        .observed_input_refs
        .iter()
        .chain(receipt.validation_evidence_refs.iter())
        .chain(receipt.human_acknowledgement.iter())
    {
        if !valid_evidence_ref(reference) {
            blockers.push("agent_review_evidence_ref_invalid".to_string());
        }
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn requires_observed_inputs(state: AgentReviewState) -> bool {
    matches!(
        state,
        AgentReviewState::EvidenceCollected
            | AgentReviewState::ValidationPassed
            | AgentReviewState::HumanAcknowledged
    )
}

fn requires_validation_evidence(state: AgentReviewState) -> bool {
    matches!(
        state,
        AgentReviewState::ValidationPassed | AgentReviewState::HumanAcknowledged
    )
}

fn valid_evidence_ref(reference: &AgentEvidenceRef) -> bool {
    !reference.kind.trim().is_empty()
        && !reference.subject.trim().is_empty()
        && valid_sha256_digest(&reference.digest)
}

fn valid_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(kind: &str) -> AgentEvidenceRef {
        AgentEvidenceRef {
            kind: kind.to_string(),
            subject: "workload-1".to_string(),
            digest: format!("sha256:{}", "a".repeat(64)),
            lineage: None,
        }
    }

    fn receipt(state: AgentReviewState) -> AgentReviewReceipt {
        AgentReviewReceipt {
            schema: AGENT_REVIEW_RECEIPT_SCHEMA.to_string(),
            workload_id: "workload-1".to_string(),
            proposal_digest: format!("sha256:{}", "b".repeat(64)),
            state,
            observed_input_refs: vec![evidence("source_observation")],
            validation_evidence_refs: vec![evidence("canonical_gate_report")],
            human_acknowledgement: Some(evidence("human_review")),
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn reviewed_receipt_admits_a_policy_that_requires_human_review() {
        assert!(
            admit_agent_review_receipt(&receipt(AgentReviewState::HumanAcknowledged), true).is_ok()
        );
    }

    #[test]
    fn validation_without_human_acknowledgement_cannot_bypass_required_review() {
        let mut candidate = receipt(AgentReviewState::ValidationPassed);
        candidate.human_acknowledgement = None;

        let blockers = admit_agent_review_receipt(&candidate, true).expect_err("must block");
        assert!(blockers.contains(&"agent_review_human_acknowledgement_required".to_string()));
        assert!(blockers.contains(&"agent_review_human_acknowledgement_missing".to_string()));
    }

    #[test]
    fn validation_evidence_is_required_before_apply_admission() {
        let mut candidate = receipt(AgentReviewState::ValidationPassed);
        candidate.validation_evidence_refs.clear();

        let blockers = admit_agent_review_receipt(&candidate, false).expect_err("must block");
        assert!(blockers.contains(&"agent_review_validation_evidence_missing".to_string()));
    }

    #[test]
    fn proposal_digest_is_bound_before_any_apply_admission() {
        let mut candidate = receipt(AgentReviewState::HumanAcknowledged);
        candidate.proposal_digest = "forged".to_string();

        let blockers = admit_agent_review_receipt(&candidate, true).expect_err("must block");
        assert!(blockers.contains(&"agent_review_proposal_digest_invalid".to_string()));
    }
}
