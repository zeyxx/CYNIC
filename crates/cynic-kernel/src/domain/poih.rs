//! Proof of Intelligence (PoIH) domain types — mint permit gating.
//!
//! A mint permit authorizes a wallet to attest intelligence on-chain.
//! Gate: behavioral_dog must return WAG or HOWL on the wallet's behavioral profile.

use serde::{Deserialize, Serialize};

/// Minimum Q-score required for a wallet to receive a mint permit.
/// WAG threshold: φ⁻² = 0.382. Permits are only issued above this.
pub const MINT_PERMIT_THRESHOLD: f64 = 0.382;

/// Verdict kinds that qualify for a mint permit (lowercase strings, for matching).
pub const PERMIT_APPROVED_KINDS: &[&str] = &["wag", "howl"];

/// Source of proof submitted with the mint permit request.
/// Declared upfront so Dogs know what behavioral evidence is available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProofSource {
    /// On-chain transaction history via Helius Enhanced Transactions.
    OnChainHistory,
    /// Chess profile from Blitz & Chill (optional enrichment).
    ChessProfile,
    /// Both on-chain history and chess profile.
    Combined,
}

/// A mint permit: signed authorization for a wallet to attest on-chain.
/// Issued when behavioral_dog returns WAG or HOWL on the wallet's profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintPermit {
    /// Wallet address that passed the behavioral gate.
    pub wallet_address: String,
    /// Proof source used for behavioral evaluation.
    pub proof_source: ProofSource,
    /// Q-score from behavioral_dog (≥ MINT_PERMIT_THRESHOLD).
    pub q_score: f64,
    /// Verdict kind: "wag" or "howl".
    pub verdict_kind: String,
    /// Always true when permit is issued (gate was passed).
    pub approved: bool,
    /// UUID for this permit — used for on-chain reference.
    pub verdict_id: String,
    /// BLAKE3 hash of permit content — integrity chain stub.
    /// Empty string until full integrity chain is wired (Task 7+).
    pub verdict_hash: String,
    /// ISO 8601 timestamp of evaluation.
    pub evaluated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_is_wag_level() {
        // MINT_PERMIT_THRESHOLD must equal the WAG threshold (φ⁻² ≈ 0.382).
        assert!((MINT_PERMIT_THRESHOLD - 0.382).abs() < 1e-6);
    }

    #[test]
    fn permit_approved_kinds_contains_wag_and_howl() {
        assert!(PERMIT_APPROVED_KINDS.contains(&"wag"));
        assert!(PERMIT_APPROVED_KINDS.contains(&"howl"));
        // Must NOT contain bark, growl, epoche — the rejection tier
        assert!(!PERMIT_APPROVED_KINDS.contains(&"bark"));
        assert!(!PERMIT_APPROVED_KINDS.contains(&"growl"));
        assert!(!PERMIT_APPROVED_KINDS.contains(&"epoche"));
    }

    #[test]
    fn mint_permit_serializes() {
        let permit = MintPermit {
            wallet_address: "11111111111111111111111111111111".to_string(),
            proof_source: ProofSource::OnChainHistory,
            q_score: 0.42,
            verdict_kind: "wag".to_string(),
            approved: true,
            verdict_id: "test-id".to_string(),
            verdict_hash: String::new(),
            evaluated_at: "2026-05-28T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&permit).expect("serialize failed");
        assert!(json.contains("wallet_address"));
        assert!(json.contains("on_chain_history"));
    }
}
