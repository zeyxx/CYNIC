//! Anchor outcome: decision → result pair for the outcome loop.
//!
//! Pure — no I/O.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorOutcome {
    pub decision_id: String,
    pub domain: String,
    pub result: String,
    pub measured_at_ms: i64,
    pub notes: Option<String>,
}

impl AnchorOutcome {
    pub fn new(
        decision_id: impl Into<String>,
        domain: impl Into<String>,
        result: impl Into<String>,
        measured_at_ms: i64,
    ) -> Self {
        Self {
            decision_id: decision_id.into(),
            domain: domain.into(),
            result: result.into(),
            measured_at_ms,
            notes: None,
        }
    }
}