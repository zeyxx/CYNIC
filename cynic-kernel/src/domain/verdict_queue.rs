//! Verdict Submission Queue — tracks verdicts pending onchain anchoring.
//!
//! Verdicts with q_score ≥ φ⁻¹ (0.618) are queued for auto-submission to Pinocchio.
//! Status: pending → submitted → confirmed (or failed).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedVerdict {
    /// Verdict ID (same as verdict.id)
    pub verdict_id: String,
    /// Verdict content hash (proposal_hash on-chain)
    pub content_hash: String,
    /// Q-Score (should be ≥ 0.618 to queue)
    pub q_score: f64,
    /// Queue status: pending, submitted, confirmed, failed
    pub status: SubmissionStatus,
    /// Solana transaction signature (if submitted)
    pub tx_signature: Option<String>,
    /// Onchain PDA address (verdict PDA)
    pub verdict_pda: Option<String>,
    /// Retry count (max 3)
    pub retry_count: u32,
    /// Last error (if status = failed)
    pub error_reason: Option<String>,
    /// Enqueued timestamp
    pub created_at: String,
    /// Last submission attempt
    pub submitted_at: Option<String>,
    /// Confirmed timestamp (when tx finalized)
    pub confirmed_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubmissionStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "submitted")]
    Submitted,
    #[serde(rename = "confirmed")]
    Confirmed,
    #[serde(rename = "failed")]
    Failed,
}

impl std::fmt::Display for SubmissionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Submitted => write!(f, "submitted"),
            Self::Confirmed => write!(f, "confirmed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}
