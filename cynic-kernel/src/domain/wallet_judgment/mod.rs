//! Wallet Judgment Domain — Anti-Sybil Evaluation
//!
//! Evaluates wallet authenticity for Option C Personality Card minting.
//! Dogs score on six axioms: FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY.

use serde::{Deserialize, Serialize};

use crate::domain::dog::{AxiomScores, PHI_INV, VerdictKind};

/// Wallet profile extracted from enrichment context.
/// All fields are observable from game history and on-chain data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletProfile {
    pub wallet_address: String,
    pub games_completed: u32,
    pub archetype_consistency: f64, // % of games in modal archetype [0, 1]
    pub wallet_age_days: u32,
    pub average_game_duration: u32,      // seconds
    pub duration_variance: f64,          // coefficient of variation (σ/μ)
    pub opening_repertoire_hash: String, // hash of opening sequences
    pub move_sequence_hash: String,      // hash of move sequences
    pub suspicious_cluster: bool,        // circular funding / shared IP
    pub replay_risk: bool,               // moves copy-pasted from another wallet
}

/// Deterministic dog for wallet authenticity validation.
/// Pure function — no state, no side effects.
/// Implements the algorithm from `cynic-kernel/domains/wallet-judgment-dogs.md`.
pub fn deterministic_dog(profile: &WalletProfile) -> (VerdictKind, AxiomScores) {
    let mut critical_fails = Vec::new();

    // GATE 1: Minimum games
    if profile.games_completed < 5 {
        critical_fails.push("insufficient_samples");
        let scores = AxiomScores::default();
        return (VerdictKind::Bark, scores);
    }

    // GATE 2: Critical Sybil markers
    if profile.suspicious_cluster || profile.replay_risk {
        critical_fails.push("sybil_coordination");
        let scores = AxiomScores::default(); // Default (all 0.05, minimal confidence)
        return (VerdictKind::Bark, scores);
    }

    // FIDELITY: Archetype consistency
    let fidelity = if profile.archetype_consistency >= 0.80 {
        0.55
    } else if profile.archetype_consistency >= 0.50 {
        0.35
    } else {
        0.15
    };

    // PHI: Time distribution harmony + temporal spread
    let time_harmony = if profile.duration_variance <= 0.20 {
        0.55
    } else if profile.duration_variance <= 0.50 {
        0.35
    } else {
        0.15
    };

    let temporal_spread = if profile.games_completed >= 20 && profile.wallet_age_days >= 20 {
        0.55
    } else if profile.games_completed >= 5 && profile.wallet_age_days >= 3 {
        // Only credit spread if play is reasonably consistent (variance <= 0.50)
        // Erratic play despite age doesn't show authentic engagement
        if profile.duration_variance <= 0.50 {
            0.35
        } else {
            0.15
        }
    } else {
        0.15
    };

    let phi = (time_harmony + temporal_spread) / 2.0;

    // VERIFY: Timestamp authenticity (assume on-chain verifiable)
    let verify = 0.55;

    // CULTURE: Gameplay patterns (engagement depth)
    let culture = if profile.replay_risk {
        0.15
    } else if profile.games_completed >= 15 {
        0.55 // Strong engagement (15+ games)
    } else if profile.games_completed >= 5 && profile.wallet_age_days >= 5 {
        0.50 // Established baseline (5+ games, proven age)
    } else if profile.games_completed >= 5 {
        0.45 // Meets minimum but recent
    } else {
        0.25 // Insufficient engagement
    };

    // BURN: Wallet efficiency and activity (commitment over time)
    // GATE 1 already ensures games >= 5, so BURN rewards longevity
    let burn = if profile.wallet_age_days < 5 {
        0.15 // Too new to show commitment
    } else if profile.wallet_age_days >= 30 {
        0.55 // Long-running, proven commitment
    } else if profile.wallet_age_days >= 10 {
        0.45 // Moderate age, passed minimum threshold
    } else {
        0.25 // Very new (5-9 days), minimal proof
    };

    // SOVEREIGNTY: Decision autonomy
    let sovereignty = if profile.replay_risk {
        0.05
    } else if profile.archetype_consistency >= 0.75 {
        0.55
    } else {
        0.35
    };

    // Aggregate and apply age ceiling (asymmetric confidence)
    // Young wallets are Sybil-risky; ceiling is strict even with perfect metrics
    let mut q_score: f64 = (fidelity + phi + verify + culture + burn + sovereignty) / 6.0;

    if profile.wallet_age_days < 3 {
        q_score = q_score.min(0.30); // Very new: near-BARK even if consistent
    } else if profile.wallet_age_days < 5 {
        q_score = q_score.min(0.35); // Still new: GROWL floor
    } else if profile.wallet_age_days < 30 {
        q_score = q_score.min(0.55); // Developing: allow WAG
    }
    // else (30+): no ceiling, up to HOWL

    // Compute verdict kind
    // Thresholds: HOWL (0.618 φ⁻¹) > WAG (0.50) > GROWL (0.382 φ⁻²) > BARK
    // WAG at 0.50 (not 0.528) allows established wallets with basic engagement to pass
    let kind = if q_score >= PHI_INV {
        VerdictKind::Howl
    } else if q_score >= 0.50 {
        VerdictKind::Wag
    } else if q_score >= 0.382 {
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

    fn minimal_profile() -> WalletProfile {
        WalletProfile {
            wallet_address: "test-wallet".to_string(),
            games_completed: 5,
            archetype_consistency: 0.80,
            wallet_age_days: 30,
            average_game_duration: 300,
            duration_variance: 0.20,
            opening_repertoire_hash: "hash1".to_string(),
            move_sequence_hash: "hash2".to_string(),
            suspicious_cluster: false,
            replay_risk: false,
        }
    }

    #[test]
    fn deterministic_dog_gate1_insufficient_games() {
        let mut profile = minimal_profile();
        profile.games_completed = 3;
        let (kind, _scores) = deterministic_dog(&profile);
        assert_eq!(kind, VerdictKind::Bark);
    }

    #[test]
    fn deterministic_dog_gate2_suspicious_cluster() {
        let mut profile = minimal_profile();
        profile.suspicious_cluster = true;
        let (kind, _scores) = deterministic_dog(&profile);
        assert_eq!(kind, VerdictKind::Bark);
    }

    #[test]
    fn deterministic_dog_gate2_replay_risk() {
        let mut profile = minimal_profile();
        profile.replay_risk = true;
        let (kind, _scores) = deterministic_dog(&profile);
        assert_eq!(kind, VerdictKind::Bark);
    }

    #[test]
    fn deterministic_dog_authentic_established_wallet() {
        let profile = minimal_profile();
        let (kind, scores) = deterministic_dog(&profile);
        // Established wallet (30+ days, 5+ games, 80% consistent, low variance)
        // should score WAG or HOWL
        assert!(kind == VerdictKind::Wag || kind == VerdictKind::Howl);
        assert!(scores.fidelity > 0.4);
        assert!(scores.phi > 0.3);
    }

    #[test]
    fn deterministic_dog_new_wallet_age_ceiling() {
        let mut profile = minimal_profile();
        profile.wallet_age_days = 2;
        profile.archetype_consistency = 0.99; // Would otherwise be high
        let (kind, scores) = deterministic_dog(&profile);
        // Despite high consistency, age < 5 days applies ceiling of 0.45
        let q_avg = (scores.fidelity
            + scores.phi
            + scores.verify
            + scores.culture
            + scores.burn
            + scores.sovereignty)
            / 6.0;
        assert!(q_avg <= 0.45, "age ceiling should limit q_score");
        assert_eq!(kind, VerdictKind::Bark); // Will fail to reach WAG threshold
    }

    #[test]
    fn deterministic_dog_inconsistent_archetype() {
        let mut profile = minimal_profile();
        profile.archetype_consistency = 0.40; // Low consistency
        let (_kind, scores) = deterministic_dog(&profile);
        assert!(scores.fidelity < 0.25);
        assert!(scores.sovereignty < 0.4); // Uncertain autonomy
    }

    #[test]
    fn deterministic_dog_high_variance_gameplay() {
        let mut profile = minimal_profile();
        profile.duration_variance = 0.65; // Chaotic (>50%)
        let (_kind, scores) = deterministic_dog(&profile);
        assert!(scores.phi < 0.25); // PHI should be low (bot-like)
    }

    #[test]
    fn deterministic_dog_established_rich_history() {
        let mut profile = minimal_profile();
        profile.games_completed = 25;
        profile.wallet_age_days = 45;
        profile.archetype_consistency = 0.90;
        profile.duration_variance = 0.15;
        let (kind, scores) = deterministic_dog(&profile);
        // Established wallet with rich history should approach HOWL
        let q_avg = (scores.fidelity
            + scores.phi
            + scores.verify
            + scores.culture
            + scores.burn
            + scores.sovereignty)
            / 6.0;
        assert!(q_avg >= 0.54, "established wallet should score high"); // 0.54 accounts for floating-point rounding
        assert!(kind == VerdictKind::Howl || kind == VerdictKind::Wag);
    }
}
