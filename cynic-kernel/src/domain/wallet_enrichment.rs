//! Wallet behavioral enrichment — temporal behavior signals from on-chain data.
//! Sovereign: computed from Helius parsed transaction history. No external APIs.
//! Signals validated on $ASDFASDFA holders (3 distinct profiles confirmed).

use serde::{Deserialize, Serialize};

/// Behavioral profile of a wallet, computed from parsed transaction history.
/// Used by the mint-permit flow to gate PoIH attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBehavioralProfile {
    pub wallet_address: String,
    /// Wallet age in days (from earliest transaction in history)
    pub wallet_age_days: u32,
    /// SOL balance
    pub sol_balance: f64,

    // ── Activity signals (from parsed tx history) ──
    /// Total SWAP transactions in analyzed history
    pub total_swaps: u32,
    /// Average swaps per day (total_swaps / days_active)
    pub swaps_per_day: f64,
    /// Number of distinct token mints traded
    pub distinct_tokens_traded: u32,

    // ── Temporal signals (the discriminating signals) ──
    /// Ratio of swaps on pump.fun (PUMP_FUN + PUMP_AMM) vs total swaps (0.0-1.0)
    /// High = degen/meme trader. Low = DeFi/blue-chip trader.
    pub pump_fun_ratio: f64,
    /// Estimated median hold duration in hours (time between buy and sell of same token)
    /// High = holder. Low = flipper/sniper.
    pub median_hold_hours: f64,
    /// Tokens bought and sold within 24 hours
    pub flip_count: u32,
    /// Tokens held longer than 7 days (still in wallet or sold after 7d)
    pub long_hold_count: u32,

    // ── Size signals ──
    /// Total volume traded in SOL (sum of SOL transfers in swaps)
    pub total_volume_sol: f64,

    // ── Portfolio state (from getTokenBalances or getAssetsByOwner) ──
    /// Current number of fungible tokens held
    pub fungible_token_count: u32,

    // ── Optional B&C enrichment ──
    pub chess_profile: Option<super::wallet_judgment::WalletProfile>,
}

/// Minimum swap count to produce meaningful behavioral signals.
/// Below this, temporal analysis is unreliable.
pub const MIN_SWAPS_FOR_ANALYSIS: u32 = 3;

/// Minimum verdict coverage for portfolio quality signal (deferred — not used in temporal analysis).
pub const MIN_VERDICT_COVERAGE: f64 = 0.1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_swaps_is_reasonable() {
        assert!(MIN_SWAPS_FOR_ANALYSIS >= 2);
        assert!(MIN_SWAPS_FOR_ANALYSIS <= 10);
    }

    #[test]
    fn profile_serializes() {
        let profile = WalletBehavioralProfile {
            wallet_address: "test".into(),
            wallet_age_days: 30,
            sol_balance: 1.5,
            total_swaps: 10,
            swaps_per_day: 0.33,
            distinct_tokens_traded: 5,
            pump_fun_ratio: 0.8,
            median_hold_hours: 48.0,
            flip_count: 2,
            long_hold_count: 3,
            total_volume_sol: 5.0,
            fungible_token_count: 8,
            chess_profile: None,
        };
        let json = serde_json::to_string(&profile).expect("serialize failed");
        assert!(json.contains("pump_fun_ratio"));
        assert!(json.contains("median_hold_hours"));
    }
}
