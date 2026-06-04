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

    // Personality Signals for 2027 Vision (Crystallized Identity)
    pub tactical_complexity: f64, // [0, 1] frequency of complex tactical sequences
    pub engine_adherence: f64,    // [0, 1] similarity to Stockfish top choices
    pub opening_theory_depth: f64, // [0, 1] average depth of book moves
    pub blitz_speed_ratio: f64,   // [0, 1] time spent vs. move quality
    pub endgame_accuracy: f64,    // [0, 1] structural correctness in late game
}

impl WalletProfile {
    /// Convert wallet profile to a structured stimulus for Dogs to evaluate.
    pub fn to_stimulus(&self) -> String {
        crate::domain::stimulus::build_wallet_stimulus(self)
    }
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

    // FIDELITY: Human Authenticity vs Engine
    // Humans aren't engines. Extreme engine adherence (>0.95) is a fidelity red flag.
    let engine_penalty: f64 = if profile.engine_adherence > 0.95 {
        0.20 // Possible engine use
    } else if profile.engine_adherence > 0.90 {
        0.10 // Suspect
    } else {
        0.0
    };

    let fidelity_base: f64 = if profile.archetype_consistency >= 0.80 {
        0.55
    } else if profile.archetype_consistency >= 0.50 {
        0.35
    } else {
        0.15
    };
    let fidelity = (fidelity_base - engine_penalty).max(0.05);

    // PHI: Structural harmony in time and late-game technique
    let time_harmony = if profile.duration_variance <= 0.20 {
        0.55
    } else if profile.duration_variance <= 0.50 {
        0.35
    } else {
        0.15
    };

    let phi = (time_harmony + profile.endgame_accuracy) / 2.0;

    // VERIFY: Timestamp authenticity + verifiable theory knowledge
    let verify = (0.55 + profile.opening_theory_depth) / 2.0;

    // CULTURE: Gameplay patterns (engagement depth + theory depth)
    let culture_base = if profile.replay_risk {
        0.15
    } else if profile.games_completed >= 15 {
        0.55
    } else if profile.games_completed >= 5 {
        0.45
    } else {
        0.25
    };
    let culture = (culture_base + profile.opening_theory_depth) / 2.0;

    // BURN: Efficiency (time spent vs quality)
    let burn = (profile.blitz_speed_ratio + (1.0 - profile.duration_variance).max(0.0)) / 2.0;

    // SOVEREIGNTY: Decision autonomy (Tactical complexity indicates human agency)
    let sovereignty = if profile.replay_risk {
        0.05
    } else {
        (profile.tactical_complexity + profile.archetype_consistency) / 2.0
    };

    // Aggregate and apply age ceiling (asymmetric confidence)
    let mut q_score: f64 = (fidelity + phi + verify + culture + burn + sovereignty) / 6.0;

    if profile.wallet_age_days < 3 {
        q_score = q_score.min(0.30);
    } else if profile.wallet_age_days < 5 {
        q_score = q_score.min(0.35);
    } else if profile.wallet_age_days < 30 {
        q_score = q_score.min(0.55);
    }

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
            tactical_complexity: 0.50,
            engine_adherence: 0.60,
            opening_theory_depth: 0.50,
            blitz_speed_ratio: 0.50,
            endgame_accuracy: 0.50,
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
        let (kind, _scores) = deterministic_dog(&profile);
        // Despite high individual scores, age < 3 days applies ceiling of 0.30 to q_score
        // ensuring a BARK verdict.
        assert_eq!(kind, VerdictKind::Bark);
    }

    #[test]
    fn deterministic_dog_inconsistent_archetype() {
        let mut profile = minimal_profile();
        profile.archetype_consistency = 0.40; // Low consistency
        profile.tactical_complexity = 0.10; // Also low
        let (_kind, scores) = deterministic_dog(&profile);
        assert!(scores.sovereignty < 0.4); // Now should be low: (0.1 + 0.4) / 2 = 0.25
    }

    #[test]
    fn deterministic_dog_high_variance_gameplay() {
        let mut profile = minimal_profile();
        profile.duration_variance = 0.65; // Chaotic (>50%)
        profile.endgame_accuracy = 0.10; // Also low
        let (_kind, scores) = deterministic_dog(&profile);
        assert!(scores.phi < 0.25); // PHI should be low: (0.15 + 0.1) / 2 = 0.125
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
