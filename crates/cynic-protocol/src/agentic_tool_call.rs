//! Foundational Agentic Tool Call (ATC) contract and protocol types.
//!
//! An Agentic Tool Call in CYNIC is an autonomous unit of intent, perception,
//! policy enforcement, monotonic budget consumption, and content-addressed proof.

use crate::agent_recursion::AgentBudget;
use crate::solana_ops::AllowanceGrant;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const AGENTIC_TOOL_CALL_INPUT_SCHEMA: &str = "cynic.agentic_tool_call_input.v1";
pub const AGENTIC_TOOL_CALL_RESULT_SCHEMA: &str = "cynic.agentic_tool_call_result.v1";

/// Execution status of an Agentic Tool Call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgenticToolCallStatus {
    Completed,
    HumanApprovalRequired,
    Rejected,
    BudgetExceeded,
    ExecutionFailed,
}

/// Typed input envelope for an Agentic Tool Call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgenticToolCallInput {
    pub schema: String,
    pub call_id: String,
    pub agent_id: String,
    pub parent_agent_id: Option<String>,
    pub node_id: String,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub budget: AgentBudget,
    pub allowance: Option<AllowanceGrant>,
    pub timestamp_unix_ms: u64,
}

impl AgenticToolCallInput {
    pub fn new(
        call_id: impl Into<String>,
        agent_id: impl Into<String>,
        node_id: impl Into<String>,
        tool_name: impl Into<String>,
        parameters: serde_json::Value,
        budget: AgentBudget,
    ) -> Self {
        Self {
            schema: AGENTIC_TOOL_CALL_INPUT_SCHEMA.to_string(),
            call_id: call_id.into(),
            agent_id: agent_id.into(),
            parent_agent_id: None,
            node_id: node_id.into(),
            tool_name: tool_name.into(),
            parameters,
            budget,
            allowance: None,
            timestamp_unix_ms: 0,
        }
    }
}

/// Typed result envelope containing outcome, risk metrics, and content-addressed proof.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgenticToolCallResult {
    pub schema: String,
    pub call_id: String,
    pub status: AgenticToolCallStatus,
    pub output: serde_json::Value,
    pub risk_score: f64,
    #[serde(default)]
    pub detected_threats: Vec<String>,
    pub proof_digest: String,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
    pub consumed_budget: AgentBudget,
    pub approval_reason: Option<String>,
}

impl AgenticToolCallResult {
    /// Computes the content-addressed cryptographic proof digest for the tool call outcome.
    pub fn compute_proof_digest(
        call_id: &str,
        status: AgenticToolCallStatus,
        output: &serde_json::Value,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(call_id.as_bytes());
        hasher.update(format!("{status:?}").as_bytes());
        hasher.update(output.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn proof_digest_is_deterministic() {
        let digest1 = AgenticToolCallResult::compute_proof_digest(
            "call_101",
            AgenticToolCallStatus::Completed,
            &json!({"amount": 10.0}),
        );
        let digest2 = AgenticToolCallResult::compute_proof_digest(
            "call_101",
            AgenticToolCallStatus::Completed,
            &json!({"amount": 10.0}),
        );

        assert_eq!(digest1, digest2);
        assert_eq!(digest1.len(), 64);
    }
}
