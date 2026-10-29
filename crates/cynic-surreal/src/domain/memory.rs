//! Functional Core / DDD Aggregate: Epistemic fact and memory records.
//! ZERO I/O: pure data structures and assertions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicStatus {
    Observed,
    Proven,
    Hypothesized,
    Contradicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicFactRecord {
    pub fact_id: String,
    pub subject: String,
    pub predicate: String,
    pub object_value: String,
    pub status: EpistemicStatus,
    pub proof_reference: Option<String>,
    pub recorded_at_epoch_ms: u64,
}

impl EpistemicFactRecord {
    pub const TABLE_NAME: &'static str = "epistemic_facts";

    pub fn new(
        fact_id: impl Into<String>,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object_value: impl Into<String>,
        status: EpistemicStatus,
        recorded_at_epoch_ms: u64,
    ) -> Self {
        Self {
            fact_id: fact_id.into(),
            subject: subject.into(),
            predicate: predicate.into(),
            object_value: object_value.into(),
            status,
            proof_reference: None,
            recorded_at_epoch_ms,
        }
    }

    pub fn with_proof(mut self, proof_reference: impl Into<String>) -> Self {
        self.proof_reference = Some(proof_reference.into());
        self
    }
}
