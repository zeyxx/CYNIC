//! Solana Pay port for the zeroclaw allowance organ.
//!
//! Pure boundary: a use case owns the allowance policy; the adapter binds
//! Solana Pay / on-chain transfer. Test fakes return fixed receipts.

use serde::{Deserialize, Serialize};

/// A bounded spending allowance granted by an owner to a delegate over a SPL
/// token, enforceable off-chain and optionally mirrored on-chain via Solana Pay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowanceGrant {
    pub owner_pubkey: String,
    pub delegate_pubkey: String,
    pub spl_token: String,
    pub max_amount_per_period: u64,
    pub period_seconds: u64,
    pub remaining_allowance: u64,
    pub allowlist_recipients: Vec<String>,
    pub allowlist_mints: Vec<String>,
    pub expires_at: Option<u64>,
}

/// A Solana Pay request produced for a delegate to redeem an allowance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolanaPayRequest {
    pub recipient: String,
    pub amount: u64,
    pub spl_token: String,
    pub allowance_id: String,
    pub memo: String,
}

/// Receipt after a Solana Pay transfer settled (or was observed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolanaPayReceipt {
    pub signature: String,
    pub slot: u64,
    pub amount: u64,
    pub allowance_id: String,
}

/// Adapter boundary for Solana Pay settlement and allowance observation.
pub trait SolanaPayPort: Send + Sync {
    /// Submit a Solana Pay transfer for an allowance redemption.
    fn submit_pay(&self, req: &SolanaPayRequest) -> Result<SolanaPayReceipt, String>;
    /// Observe the current on-chain (or off-chain mirror) allowance state.
    fn observe_allowance(&self, allowance_id: &str) -> Result<AllowanceGrant, String>;
}
