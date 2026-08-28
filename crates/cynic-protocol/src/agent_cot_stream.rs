//! Chain-of-Thought streaming events for real-time agent supervision.
//!
//! Schema: cynic.agent_cot_stream_event.v1

use serde::{Deserialize, Serialize};

pub const AGENT_COT_STREAM_EVENT_SCHEMA: &str = "cynic.agent_cot_stream_event.v1";

/// Nature of the emitted chunk in the reasoning stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CotChunkKind {
    /// Internal reasoning fragment (thinking).
    ReasoningThinking,
    /// Structured tool call plan (tool_call).
    ToolCallPlan,
    /// Final response for the user.
    FinalOutput,
}

/// Real-time alignment supervisor disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum CotSupervisorDisposition {
    /// Flow conforms to invariants.
    Admit,
    /// Suspicious pattern detected (risk score, not immediate block).
    Warn { reason: String, risk_score: f32 },
    /// Critical alignment break: triggers slot abortion.
    Abort {
        reason: String,
        violated_invariant: String,
    },
}

/// Telemetry event emitted by the Inference Gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCotStreamEvent {
    pub schema: String,
    pub session_id: String,
    pub agent_id: String,
    pub sequence_number: u64,
    pub timestamp_unix_ms: u64,
    pub chunk_kind: CotChunkKind,
    pub token_text: String,
    pub cumulative_tokens: u32,
    /// Draft acceptance flag for speculative decoding (Axe B).
    pub draft_accepted: Option<bool>,
    pub disposition: CotSupervisorDisposition,
}

impl AgentCotStreamEvent {
    pub fn admit(
        session_id: &str,
        agent_id: &str,
        seq: u64,
        kind: CotChunkKind,
        text: &str,
    ) -> Self {
        Self {
            schema: AGENT_COT_STREAM_EVENT_SCHEMA.to_string(),
            session_id: session_id.to_string(),
            agent_id: agent_id.to_string(),
            sequence_number: seq,
            timestamp_unix_ms: 0,
            chunk_kind: kind,
            token_text: text.to_string(),
            cumulative_tokens: 0,
            draft_accepted: None,
            disposition: CotSupervisorDisposition::Admit,
        }
    }

    pub fn abort(
        session_id: &str,
        agent_id: &str,
        seq: u64,
        reason: &str,
        invariant: &str,
    ) -> Self {
        Self {
            schema: AGENT_COT_STREAM_EVENT_SCHEMA.to_string(),
            session_id: session_id.to_string(),
            agent_id: agent_id.to_string(),
            sequence_number: seq,
            timestamp_unix_ms: 0,
            chunk_kind: CotChunkKind::ReasoningThinking,
            token_text: String::new(),
            cumulative_tokens: 0,
            draft_accepted: None,
            disposition: CotSupervisorDisposition::Abort {
                reason: reason.to_string(),
                violated_invariant: invariant.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_event_has_admit_disposition() {
        let e = AgentCotStreamEvent::abort("s1", "a1", 1, "test", "inv1");
        match e.disposition {
            CotSupervisorDisposition::Abort { .. } => {}
            _ => panic!("expected Abort"),
        }
    }

    #[test]
    fn abort_event_has_abort_disposition() {
        let e = AgentCotStreamEvent::abort("s1", "a1", 1, "evasion", "no_escape");
        match &e.disposition {
            CotSupervisorDisposition::Abort { reason, .. } => {
                assert_eq!(reason, "evasion");
            }
            _ => panic!("expected Abort"),
        }
    }
}
