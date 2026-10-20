use serde::{Deserialize, Serialize};

use cynic_protocol::evidence::valid_sha256_digest;
use cynic_traces::session::NormalizedSession;

pub const AGENT_EXECUTION_RECEIPT_SCHEMA: &str = "cynic.agent_execution_receipt.v1";

/// A content-addressed fact observed at a bounded agent execution boundary.
///
/// This is a join record, not a second agent log: it correlates the session,
/// delegation, workload, lease, source identity, and evidence already owned by
/// their respective protocols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionReceipt {
    pub schema: String,
    pub receipt_id: String,
    pub kind: AgentExecutionReceiptKind,
    pub agent_id: String,
    pub session_id: String,
    pub node_id: String,
    pub delegation_id: Option<String>,
    pub workload_id: Option<String>,
    pub lease_id: Option<String>,
    pub fencing_token: Option<u64>,
    pub repository_id: Option<String>,
    pub base_commit: Option<String>,
    pub evidence_digest: String,
    pub previous_receipt_digest: Option<String>,
    pub observed_at_unix_ms: u64,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionReceiptKind {
    SessionIssued,
    SessionRevoked,
    DelegationAdmitted,
    WorkspaceClaimed,
    WorkloadDispatched,
    ActionObserved,
    SourceReadBackObserved,
    WorkloadCompleted,
}

pub fn validate_agent_execution_receipt(receipt: &AgentExecutionReceipt) -> Vec<String> {
    let mut violations = Vec::new();
    if receipt.schema != AGENT_EXECUTION_RECEIPT_SCHEMA {
        violations.push("invalid_agent_execution_receipt_schema".to_string());
    }
    for (field, value) in [
        ("receipt_id", receipt.receipt_id.as_str()),
        ("agent_id", receipt.agent_id.as_str()),
        ("session_id", receipt.session_id.as_str()),
        ("node_id", receipt.node_id.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(format!("agent_execution_receipt_{field}_missing"));
        }
    }
    if receipt.observed_at_unix_ms == 0 {
        violations.push("agent_execution_receipt_observed_at_missing".to_string());
    }
    if !valid_sha256_digest(&receipt.evidence_digest) {
        violations.push("agent_execution_receipt_evidence_digest_invalid".to_string());
    }
    if receipt
        .previous_receipt_digest
        .as_deref()
        .is_some_and(|digest| !valid_sha256_digest(digest))
    {
        violations.push("agent_execution_receipt_predecessor_digest_invalid".to_string());
    }

    let execution_bound = matches!(
        receipt.kind,
        AgentExecutionReceiptKind::WorkspaceClaimed
            | AgentExecutionReceiptKind::WorkloadDispatched
            | AgentExecutionReceiptKind::ActionObserved
            | AgentExecutionReceiptKind::SourceReadBackObserved
            | AgentExecutionReceiptKind::WorkloadCompleted
    );
    if execution_bound {
        required(
            &mut violations,
            "delegation_id",
            receipt.delegation_id.as_deref(),
        );
        required(
            &mut violations,
            "workload_id",
            receipt.workload_id.as_deref(),
        );
    }
    if matches!(
        receipt.kind,
        AgentExecutionReceiptKind::WorkspaceClaimed
            | AgentExecutionReceiptKind::ActionObserved
            | AgentExecutionReceiptKind::SourceReadBackObserved
            | AgentExecutionReceiptKind::WorkloadCompleted
    ) {
        required(&mut violations, "lease_id", receipt.lease_id.as_deref());
        if receipt.fencing_token.is_none() {
            violations.push("agent_execution_receipt_fencing_token_missing".to_string());
        }
    }
    if matches!(
        receipt.kind,
        AgentExecutionReceiptKind::SourceReadBackObserved
            | AgentExecutionReceiptKind::WorkloadCompleted
    ) {
        required(
            &mut violations,
            "repository_id",
            receipt.repository_id.as_deref(),
        );
        required(
            &mut violations,
            "base_commit",
            receipt.base_commit.as_deref(),
        );
    }
    violations
}

fn required(violations: &mut Vec<String>, field: &str, value: Option<&str>) {
    if value.is_none_or(|value| value.trim().is_empty()) {
        violations.push(format!("agent_execution_receipt_{field}_missing"));
    }
}

/// The trace shape of one execution receipt, joined to its normalized session.
///
/// A receipt observes an execution boundary; a normalized session observes
/// what the agent actually did. Neither alone answers "was this session
/// healthy?". This projection joins them on the session identifier and
/// reports only what was observed: an absent trace is an explicit unknown,
/// never a zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionTraceProjection {
    pub schema: String,
    pub session_id: String,
    pub harness: String,
    pub turns: usize,
    pub tools: usize,
    pub reasoning: usize,
    pub human: usize,
    pub corrections: usize,
}

pub const AGENT_EXECUTION_TRACE_PROJECTION_SCHEMA: &str = "cynic.agent_execution_trace_projection.v1";

/// Project a receipt onto the normalized trace of its session.
///
/// Returns `None` when no normalized session matches the receipt's
/// `session_id`. A missing trace is an unknown, not an empty session: the
/// caller decides what to do with an absent projection, this function never
/// fabricates one.
pub fn project_execution_trace(
    receipt: &AgentExecutionReceipt,
    sessions: &[NormalizedSession],
) -> Option<AgentExecutionTraceProjection> {
    let session = sessions
        .iter()
        .find(|session| session.session_uid == receipt.session_id)?;
    let stats = session.stats();
    Some(AgentExecutionTraceProjection {
        schema: AGENT_EXECUTION_TRACE_PROJECTION_SCHEMA.to_string(),
        session_id: receipt.session_id.clone(),
        harness: session.harness.clone(),
        turns: stats.turns,
        tools: stats.tools,
        reasoning: stats.reasoning,
        human: stats.human,
        corrections: stats.corrections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn source_read_back() -> AgentExecutionReceipt {
        AgentExecutionReceipt {
            schema: AGENT_EXECUTION_RECEIPT_SCHEMA.to_string(),
            receipt_id: "receipt-1".to_string(),
            kind: AgentExecutionReceiptKind::SourceReadBackObserved,
            agent_id: "agent-1".to_string(),
            session_id: "session-1".to_string(),
            node_id: "node-1".to_string(),
            delegation_id: Some("delegation-1".to_string()),
            workload_id: Some("workload-1".to_string()),
            lease_id: Some("lease-1".to_string()),
            fencing_token: Some(7),
            repository_id: Some("cynic".to_string()),
            base_commit: Some("c26e4d73".to_string()),
            evidence_digest: digest('a'),
            previous_receipt_digest: Some(digest('b')),
            observed_at_unix_ms: 1,
            residual_unknowns: Vec::new(),
        }
    }

    #[test]
    fn source_read_back_requires_the_execution_join_keys() {
        assert!(validate_agent_execution_receipt(&source_read_back()).is_empty());

        let mut missing = source_read_back();
        missing.fencing_token = None;
        missing.base_commit = None;
        let violations = validate_agent_execution_receipt(&missing);
        assert!(violations.contains(&"agent_execution_receipt_fencing_token_missing".to_string()));
        assert!(violations.contains(&"agent_execution_receipt_base_commit_missing".to_string()));
    }

    #[test]
    fn session_receipts_do_not_claim_a_workload_join() {
        let receipt = AgentExecutionReceipt {
            kind: AgentExecutionReceiptKind::SessionRevoked,
            delegation_id: None,
            workload_id: None,
            lease_id: None,
            fencing_token: None,
            repository_id: None,
            base_commit: None,
            ..source_read_back()
        };
        assert!(validate_agent_execution_receipt(&receipt).is_empty());
    }

    fn normalized_session(session_uid: &str) -> NormalizedSession {
        NormalizedSession {
            session_uid: session_uid.to_string(),
            harness: "hermes".to_string(),
            source_ref: "capture.db".to_string(),
            turns: Vec::new(),
            tools: Vec::new(),
            reasoning: Vec::new(),
            edges: Vec::new(),
            states: Vec::new(),
            human: Vec::new(),
            native_meta: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn a_receipt_projects_onto_its_own_normalized_session() {
        let receipt = source_read_back();
        let sessions = vec![
            normalized_session("session-other"),
            normalized_session("session-1"),
        ];
        let projection = project_execution_trace(&receipt, &sessions)
            .expect("the session matching the receipt must project");
        assert_eq!(projection.schema, AGENT_EXECUTION_TRACE_PROJECTION_SCHEMA);
        assert_eq!(projection.session_id, "session-1");
        assert_eq!(projection.harness, "hermes");
    }

    #[test]
    fn an_absent_trace_is_an_unknown_never_a_zeroed_session() {
        let receipt = source_read_back();
        assert!(project_execution_trace(&receipt, &[]).is_none());
        assert!(project_execution_trace(&receipt, &[normalized_session("session-other")]).is_none());
    }
}
