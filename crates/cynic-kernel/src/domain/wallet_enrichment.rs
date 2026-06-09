//! Wallet behavioral enrichment — temporal behavior signals from on-chain data.
//! Sovereign: computed from Helius parsed transaction history. No external APIs.
//! Signals validated on $ASDFASDFA holders (3 distinct profiles confirmed).

use serde::{Deserialize, Serialize};

use crate::domain::dog::{AxiomScores, PHI_INV, VerdictKind};

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

/// Deterministic dog for wallet behavioral analysis.
/// Scores a WalletBehavioralProfile on 6 axioms using temporal signals.
/// Pure function — no I/O, no state.
///
/// Returns EPOCHÉ if insufficient data (young wallet + few swaps).
/// Returns BARK for high flip_count + high pump_fun_ratio (sniper/bot).
/// Returns WAG/HOWL for old wallets with low churn (diamond hands).
pub fn behavioral_dog(profile: &WalletBehavioralProfile) -> (VerdictKind, AxiomScores) {
    // ── EPOCHÉ gate: not enough data ──
    // Young wallet (<7d) + few swaps (<MIN) = can't distinguish diamond hands from ghost wallet.
    // Old wallet (≥30d) + zero swaps = diamond hands, not missing data.
    if profile.total_swaps < MIN_SWAPS_FOR_ANALYSIS && profile.wallet_age_days < 7 {
        return (VerdictKind::Epoche, AxiomScores::default());
    }

    // ── FIDELITY: Does the wallet show genuine engagement? ──
    // Long holds + low flip rate = genuine. High flip rate = extractive.
    let fidelity = if profile.flip_count == 0 && profile.wallet_age_days >= 30 {
        0.55 // Diamond hands — never flips; commitment is verifiable across months
    } else if profile.flip_count <= 2 && profile.median_hold_hours > 48.0 {
        0.45 // Occasional trader, mostly holds; 48h threshold = weekend+ horizon
    } else if profile.flip_count > 5 && profile.median_hold_hours < 1.0 {
        0.10 // Sniper — buys and dumps within minutes; extractive, not genuine
    } else if profile.flip_count > 3 {
        0.20 // Active flipper; velocity exceeds plausible conviction
    } else {
        0.35 // Moderate activity; insufficient signal to commit higher
    };

    // ── PHI: Is the behavior proportional/harmonious? ──
    // Balanced portfolio + reasonable trade frequency = harmonious.
    // Extreme concentration or extreme churn = disharmonious.
    let phi = if profile.swaps_per_day > 20.0 {
        0.10 // Bot-like frequency; >20 swaps/day exceeds human capacity for discretion
    } else if profile.swaps_per_day > 5.0 {
        0.20 // Very active, borderline automated; 5+/day strains proportionality
    } else if profile.swaps_per_day > 0.0 && profile.swaps_per_day <= 2.0 {
        0.50 // Human-scale activity; 0–2 swaps/day fits deliberate decision-making
    } else if profile.total_swaps == 0 && profile.wallet_age_days >= 30 {
        0.55 // Zen — buy and hold, zero noise; structural harmony at maximum
    } else {
        0.35 // Moderate; mixed signals
    };

    // ── VERIFY: Can the behavior be verified on-chain? ──
    // Always high — everything is on-chain. Slight penalty if very few transactions
    // (harder to establish a repeatable pattern).
    let verify = if profile.total_swaps >= 10 {
        0.55 // Strong verifiable pattern; 10+ data points establish a behavioral signature
    } else if profile.total_swaps >= MIN_SWAPS_FOR_ANALYSIS {
        0.45 // Enough to verify; sparse but real
    } else {
        0.35 // Minimal data — on-chain but single/dual events are near-anecdotal
    };

    // ── CULTURE: Does the wallet operate within ecosystem norms? ──
    // pump.fun dominance = meme/degen culture (not negative per se, but less established).
    // Diverse DEX usage = established DeFi participant.
    let culture = if profile.pump_fun_ratio > 0.9 && profile.total_swaps > 10 {
        0.15 // Pure pump.fun degen; >90% on meme launchpad across 10+ swaps = monoculture
    } else if profile.pump_fun_ratio > 0.7 {
        0.25 // Mostly meme trading; limited ecosystem engagement
    } else if profile.pump_fun_ratio < 0.3 && profile.distinct_tokens_traded >= 3 {
        0.50 // Diverse DeFi user; cross-protocol engagement signals ecosystem citizenship
    } else if profile.total_swaps == 0 && profile.wallet_age_days >= 30 {
        0.45 // Passive holder — respects the culture of conviction
    } else {
        0.35 // Mixed; no dominant signal
    };

    // ── BURN: Is the wallet efficient? (minimal waste) ──
    // Low trade count relative to volume = efficient high-conviction use.
    // Many tiny trades = gas wasting.
    let burn = if profile.total_volume_sol > 1.0 && profile.total_swaps <= 10 {
        0.50 // High conviction, few trades; meaningful SOL moved without churn
    } else if profile.total_swaps > 20 && profile.total_volume_sol < 0.5 {
        0.15 // Many tiny trades — gas wasting; 20+ swaps for <0.5 SOL total
    } else if profile.total_swaps == 0 {
        0.55 // Zero waste — ultimate burn efficiency; no gas burned on speculation
    } else {
        0.35 // Normal efficiency; no dominant pattern
    };

    // ── SOVEREIGNTY: Does the wallet maintain autonomy? ──
    // Diverse holdings + independent timing = sovereign.
    // Follows pump.fun launches in real-time = herd behavior.
    let sovereignty = if profile.swaps_per_day > 10.0 && profile.pump_fun_ratio > 0.8 {
        0.10 // Bot/copy-trader — no sovereignty; reactive to launchpad noise
    } else if profile.wallet_age_days >= 30 && profile.flip_count <= 1 {
        0.55 // Long-term independent holder; 30d+ commitment with ≤1 flip = own thesis
    } else if profile.distinct_tokens_traded >= 5 && profile.pump_fun_ratio < 0.5 {
        0.45 // Diverse, independent choices; breadth across protocols signals autonomy
    } else {
        0.30 // Moderate sovereignty; insufficient signal
    };

    // ── Aggregate (arithmetic mean — mirrors wallet_judgment pattern) ──
    let q_score: f64 = (fidelity + phi + verify + culture + burn + sovereignty) / 6.0;

    // Age ceiling (asymmetric confidence — young wallets are Sybil-risky)
    let q_capped = if profile.wallet_age_days < 3 {
        q_score.min(0.25) // Very new — even good signals might be fake
    } else if profile.wallet_age_days < 7 {
        q_score.min(0.35) // Still new — suppress overconfidence
    } else if profile.wallet_age_days < 30 {
        q_score.min(0.50) // Developing — allow WAG but not HOWL
    } else {
        q_score // No ceiling for established wallets
    };

    let kind = if q_capped >= PHI_INV {
        VerdictKind::Howl
    } else if q_capped >= 0.382 {
        VerdictKind::Wag
    } else if q_capped >= 0.236 {
        VerdictKind::Growl
    } else {
        VerdictKind::Bark
    };

    let scores = AxiomScores {
        fidelity,
        phi,
        verify,
        culture,
        burn,
        sovereignty,
        reasoning: Default::default(),
        ..Default::default()
    };

    (kind, scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_swaps_is_reasonable() {
        assert!((2..=10).contains(&MIN_SWAPS_FOR_ANALYSIS));
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

    // ── behavioral_dog fixture profiles ─────────────────────────────────────

    fn diamond_hands_profile() -> WalletBehavioralProfile {
        WalletBehavioralProfile {
            wallet_address: "diamond".into(),
            wallet_age_days: 300, // 10 months — long-term holder
            sol_balance: 5.0,
            total_swaps: 0, // never traded
            swaps_per_day: 0.0,
            distinct_tokens_traded: 0,
            pump_fun_ratio: 0.0,
            median_hold_hours: 0.0,
            flip_count: 0, // never flipped
            long_hold_count: 0,
            total_volume_sol: 0.0,
            fungible_token_count: 1,
            chess_profile: None,
        }
    }

    fn degen_trader_profile() -> WalletBehavioralProfile {
        WalletBehavioralProfile {
            wallet_address: "degen".into(),
            wallet_age_days: 30,
            sol_balance: 0.5,
            total_swaps: 35,     // active trader
            swaps_per_day: 10.0, // high frequency (borderline automated)
            distinct_tokens_traded: 8,
            pump_fun_ratio: 1.0,    // 100% pump.fun
            median_hold_hours: 0.5, // holds <1h on average — fast flipper
            flip_count: 6,          // >5 flips → extractive
            long_hold_count: 1,     // holds 1 token long-term (the ASDF signal)
            total_volume_sol: 3.0,
            fungible_token_count: 4,
            chess_profile: None,
        }
    }

    fn flipper_profile() -> WalletBehavioralProfile {
        WalletBehavioralProfile {
            wallet_address: "flipper".into(),
            wallet_age_days: 5, // young wallet
            sol_balance: 0.1,
            total_swaps: 50,     // high churn on a new wallet
            swaps_per_day: 25.0, // >20 = bot-like
            distinct_tokens_traded: 20,
            pump_fun_ratio: 0.95,   // pure pump.fun
            median_hold_hours: 0.1, // dumps in minutes
            flip_count: 15,         // extreme flip count
            long_hold_count: 0,
            total_volume_sol: 0.3, // <0.5 SOL across 50 swaps = gas wasting
            fungible_token_count: 2,
            chess_profile: None,
        }
    }

    // ── behavioral_dog tests ────────────────────────────────────────────────

    #[test]
    fn diamond_hands_scores_wag_or_howl() {
        let (kind, scores) = behavioral_dog(&diamond_hands_profile());
        assert!(
            kind == VerdictKind::Wag || kind == VerdictKind::Howl,
            "Diamond hands should be WAG or HOWL, got {kind:?}"
        );
        assert!(scores.fidelity >= 0.5, "fidelity={}", scores.fidelity);
        assert!(
            scores.sovereignty >= 0.5,
            "sovereignty={}",
            scores.sovereignty
        );
    }

    #[test]
    fn flipper_scores_bark() {
        let (kind, scores) = behavioral_dog(&flipper_profile());
        assert_eq!(kind, VerdictKind::Bark, "Flipper should BARK");
        assert!(scores.fidelity <= 0.20, "fidelity={}", scores.fidelity);
        assert!(
            scores.sovereignty <= 0.15,
            "sovereignty={}",
            scores.sovereignty
        );
    }

    #[test]
    fn degen_scores_growl_or_bark() {
        let (kind, _scores) = behavioral_dog(&degen_trader_profile());
        assert!(
            kind == VerdictKind::Growl || kind == VerdictKind::Bark,
            "Degen trader should be GROWL or BARK, got {kind:?}"
        );
    }

    #[test]
    fn new_wallet_few_swaps_epoche() {
        let mut profile = diamond_hands_profile();
        profile.wallet_age_days = 2; // <7 days
        profile.total_swaps = 1; // <MIN_SWAPS_FOR_ANALYSIS
        let (kind, _) = behavioral_dog(&profile);
        assert_eq!(kind, VerdictKind::Epoche, "New wallet + few swaps = EPOCHÉ");
    }

    #[test]
    fn old_wallet_zero_swaps_not_epoche() {
        let profile = diamond_hands_profile(); // 300 days, 0 swaps
        let (kind, _) = behavioral_dog(&profile);
        assert_ne!(
            kind,
            VerdictKind::Epoche,
            "Old wallet + 0 swaps = diamond hands, not EPOCHÉ"
        );
    }

    #[test]
    fn age_ceiling_caps_young_wallet() {
        let mut profile = diamond_hands_profile();
        profile.wallet_age_days = 2; // very new
        profile.total_swaps = 5; // enough data (≥ MIN_SWAPS_FOR_ANALYSIS)
        profile.flip_count = 0;
        let (kind, _) = behavioral_dog(&profile);
        // Even with good signals, age < 3 days caps at 0.25 → BARK
        assert!(
            kind == VerdictKind::Bark || kind == VerdictKind::Growl,
            "Young wallet should be capped, got {kind:?}"
        );
    }

    #[test]
    fn scores_never_exceed_phi_inv() {
        // Verify the hardcoded literal scores all stay within φ⁻¹ = 0.618
        for profile in [
            diamond_hands_profile(),
            degen_trader_profile(),
            flipper_profile(),
        ] {
            let (_, scores) = behavioral_dog(&profile);
            assert!(
                scores.fidelity <= 0.618_034,
                "fidelity={} exceeds φ⁻¹",
                scores.fidelity
            );
            assert!(scores.phi <= 0.618_034, "phi={} exceeds φ⁻¹", scores.phi);
            assert!(
                scores.verify <= 0.618_034,
                "verify={} exceeds φ⁻¹",
                scores.verify
            );
            assert!(
                scores.culture <= 0.618_034,
                "culture={} exceeds φ⁻¹",
                scores.culture
            );
            assert!(scores.burn <= 0.618_034, "burn={} exceeds φ⁻¹", scores.burn);
            assert!(
                scores.sovereignty <= 0.618_034,
                "sovereignty={} exceeds φ⁻¹",
                scores.sovereignty
            );
        }
    }
}
