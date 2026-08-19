//! Sovereign Multi-Agent Debate and Consensus Subsystem.
//!
//! Enables round-robin or adversarial debate between specialized agent models
//! (e.g., LFM2.5-2.6B, Qwen3.5-9B, MiniCPM-1B) with structured consensus verification
//! and proof generation under CYNIC coherence laws.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DebateRole {
    Proposer,
    Critic,
    Arbiter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateParticipant {
    pub model_alias: String,
    pub role: DebateRole,
    pub reasoning_budget: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateTurn {
    pub turn_index: u32,
    pub speaker: DebateParticipant,
    pub prompt: String,
    pub response: String,
    pub ttft_ms: u64,
    pub gen_tok_s: f64,
    pub timestamp_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateSession {
    pub schema: &'static str,
    pub session_id: String,
    pub topic: String,
    pub participants: Vec<DebateParticipant>,
    pub turns: Vec<DebateTurn>,
    pub consensus_reached: bool,
    pub final_consensus_text: Option<String>,
}

impl DebateSession {
    pub const SCHEMA: &'static str = "cynic.inference.multi_agent_debate.v1";

    pub fn new(session_id: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            schema: Self::SCHEMA,
            session_id: session_id.into(),
            topic: topic.into(),
            participants: Vec::new(),
            turns: Vec::new(),
            consensus_reached: false,
            final_consensus_text: None,
        }
    }

    pub fn add_participant(&mut self, model_alias: impl Into<String>, role: DebateRole, reasoning_budget: Option<u32>) {
        self.participants.push(DebateParticipant {
            model_alias: model_alias.into(),
            role,
            reasoning_budget,
        });
    }

    pub fn record_turn(&mut self, speaker: DebateParticipant, prompt: String, response: String, ttft_ms: u64, gen_tok_s: f64) {
        let turn_index = self.turns.len() as u32 + 1;
        let timestamp_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.turns.push(DebateTurn {
            turn_index,
            speaker,
            prompt,
            response,
            ttft_ms,
            gen_tok_s,
            timestamp_unix_ms,
        });
    }

    pub fn finalize_consensus(&mut self, consensus_text: String) {
        self.consensus_reached = true;
        self.final_consensus_text = Some(consensus_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debate_session_lifecycle() {
        let mut session = DebateSession::new("test-session-1", "Refactor cynic-kernel boundaries");
        session.add_participant("lfm25-2b-q8", DebateRole::Proposer, None);
        session.add_participant("qwen35-9b-iq2", DebateRole::Critic, Some(1024));

        assert_eq!(session.participants.len(), 2);
        assert!(!session.consensus_reached);

        session.record_turn(
            session.participants[0].clone(),
            "Propose plan".into(),
            "Plan proposal".into(),
            350,
            13.9,
        );

        session.record_turn(
            session.participants[1].clone(),
            "Review proposal".into(),
            "Critique: looks good".into(),
            6500,
            5.7,
        );

        session.finalize_consensus("Approved plan".into());
        assert!(session.consensus_reached);
        assert_eq!(session.turns.len(), 2);
    }
}
