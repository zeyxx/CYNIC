//! Shared spend-allowance vocabulary.
//!
//! Used by agentic tool-call envelopes (protocol) and Solana Pay operations
//! (`cynic-solana-pay`). Lives in the protocol residue because it is
//! cross-family vocabulary, not Solana-specific.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowanceGrant {
    pub grant_id: String,
    pub owner_pubkey: String,
    pub delegate_pubkey: String,
    pub spl_token: Option<String>,
    pub max_amount_per_period: f64,
    pub period_seconds: u64,
    pub remaining_allowance: f64,
    pub allowlist_recipients: Vec<String>,
    pub allowlist_mints: Vec<String>,
    pub expires_at: u64,
}
