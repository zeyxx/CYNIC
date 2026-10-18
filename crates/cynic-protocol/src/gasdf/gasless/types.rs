//! GASdf gasless relay types — pure vocabulary (zero I/O).
//!
//! Phase 0 scope: one sponsored transaction. No conversion, no burn worker,
//! no token gating. These land first; everything else is gated by measured
//! baselines (see docs/design/GASDF_BASELINE_20261016.md).

use serde::{Deserialize, Serialize};

pub const GASDF_GASLESS_SESSION_SCHEMA: &str = "cynic.gasdf.gasless.session.v1";
pub const GASDF_GASLESS_QUOTE_SCHEMA: &str = "cynic.gasdf.gasless.quote.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GasSessionStatus {
    Open,
    Quoted,
    Pending,
    Settled,
    Expired,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasSession {
    pub schema: String,
    pub session_id: String,
    pub buyer_pubkey: String,
    pub status: GasSessionStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub quote_id: Option<String>,
    pub signature: Option<String>,
    pub settled_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasQuote {
    pub schema: String,
    pub quote_id: String,
    pub session_id: String,
    pub fee_lamports: u64,
    pub gas_sponsored: bool,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeePayer {
    pub pubkey: String,
    pub balance_lamports: u64,
    pub low_balance_threshold_lamports: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeePayerHealth {
    Healthy,
    LowBalance,
    Depleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GasQuoteDecision {
    Accept { fee_lamports: u64, gas_sponsored: bool },
    Reject { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitDecision {
    Accept { fee_lamports: u64 },
    PendingConfirmation,
    Reject { reason: String },
}