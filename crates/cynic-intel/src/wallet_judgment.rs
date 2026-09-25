//! Deterministic wallet authenticity model used by the wallet-judgment dog.
//!
//! The kernel owns projection/targets; `cynic-intel` owns the scoring law.

use serde::{Deserialize, Serialize};

const PHI_INV: f64 = 0.618_033_988_749_894_8;
const PHI_INV2: f64 = 0.381_966_011_250_105;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletProfile {
    pub wallet_address: String,
    pub games_completed: u32,
    pub archetype_consistency: f64,
    pub wallet_age_days: u32,
    pub average_game_duration: u32,
    pub duration_variance: f64,
    pub opening_repertoire_hash: String,
    pub move_sequence_hash: String,
    pub suspicious_cluster: bool,
    pub replay_risk: bool,
    pub tactical_complexity: f64,
    pub engine_adherence: f64,
    pub opening_theory_depth: f64,
    pub blitz_speed_ratio: f64,
    pub endgame_accuracy: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterministicDogKind {
    Howl,
    Wag,
    Growl,
    Bark,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalletAxiomScores {
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

/// Deterministic wallet judgment used by the wallet-judgment dog.
pub fn deterministic_dog(profile: &WalletProfile) -> (DeterministicDogKind, WalletAxiomScores) {
    if profile.games_completed < 5 {
        return (
            DeterministicDogKind::Bark,
            WalletAxiomScores {
                fidelity: 0.0,
                phi: 0.0,
                verify: 0.0,
                culture: 0.0,
                burn: 0.0,
                sovereignty: 0.0,
            },
        );
    }

    if profile.suspicious_cluster || profile.replay_risk {
        return (
            DeterministicDogKind::Bark,
            WalletAxiomScores {
                fidelity: 0.0,
                phi: 0.0,
                verify: 0.0,
                culture: 0.0,
                burn: 0.0,
                sovereignty: 0.0,
            },
        );
    }

    let engine_penalty: f64 = if profile.engine_adherence > 0.95 {
        0.20
    } else if profile.engine_adherence > 0.90 {
        0.10
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

    let time_harmony = if profile.duration_variance <= 0.20 {
        0.55
    } else if profile.duration_variance <= 0.50 {
        0.35
    } else {
        0.15
    };

    let phi = (time_harmony + profile.endgame_accuracy) / 2.0;
    let verify = (0.55 + profile.opening_theory_depth) / 2.0;

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

    let burn = (profile.blitz_speed_ratio + (1.0 - profile.duration_variance).max(0.0)) / 2.0;

    let sovereignty = if profile.replay_risk {
        0.05
    } else {
        (profile.tactical_complexity + profile.archetype_consistency) / 2.0
    };

    let mut q_score = (fidelity + phi + verify + culture + burn + sovereignty) / 6.0;

    if profile.wallet_age_days < 3 {
        q_score = q_score.min(0.30);
    } else if profile.wallet_age_days < 5 {
        q_score = q_score.min(0.35);
    } else if profile.wallet_age_days < 30 {
        q_score = q_score.min(0.55);
    }

    let kind = if q_score >= PHI_INV {
        DeterministicDogKind::Howl
    } else if q_score >= 0.50 {
        DeterministicDogKind::Wag
    } else if q_score >= PHI_INV2 {
        DeterministicDogKind::Growl
    } else {
        DeterministicDogKind::Bark
    };

    (
        kind,
        WalletAxiomScores {
            fidelity,
            phi,
            verify,
            culture,
            burn,
            sovereignty,
        },
    )
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
        assert_eq!(kind, DeterministicDogKind::Bark);
    }

    #[test]
    fn deterministic_dog_gate2_suspicious_cluster() {
        let mut profile = minimal_profile();
        profile.suspicious_cluster = true;
        let (kind, _scores) = deterministic_dog(&profile);
        assert_eq!(kind, DeterministicDogKind::Bark);
    }

    #[test]
    fn deterministic_dog_gate2_replay_risk() {
        let mut profile = minimal_profile();
        profile.replay_risk = true;
        let (kind, _scores) = deterministic_dog(&profile);
        assert_eq!(kind, DeterministicDogKind::Bark);
    }

    #[test]
    fn deterministic_dog_authentic_established_wallet() {
        let profile = minimal_profile();
        let (kind, scores) = deterministic_dog(&profile);
        assert!(kind == DeterministicDogKind::Wag || kind == DeterministicDogKind::Howl);
        assert!(scores.fidelity > 0.4);
        assert!(scores.phi > 0.3);
    }

    #[test]
    fn deterministic_dog_new_wallet_age_ceiling() {
        let mut profile = minimal_profile();
        profile.wallet_age_days = 2;
        profile.archetype_consistency = 0.99;
        let (kind, _scores) = deterministic_dog(&profile);
        assert_eq!(kind, DeterministicDogKind::Bark);
    }

    #[test]
    fn deterministic_dog_inconsistent_archetype() {
        let mut profile = minimal_profile();
        profile.archetype_consistency = 0.40;
        profile.tactical_complexity = 0.10;
        let (_kind, scores) = deterministic_dog(&profile);
        assert!(scores.sovereignty < 0.4);
    }

    #[test]
    fn deterministic_dog_high_variance_gameplay() {
        let mut profile = minimal_profile();
        profile.duration_variance = 0.65;
        profile.endgame_accuracy = 0.10;
        let (_kind, scores) = deterministic_dog(&profile);
        assert!(scores.phi < 0.25);
    }

    #[test]
    fn deterministic_dog_established_rich_history() {
        let mut profile = minimal_profile();
        profile.games_completed = 25;
        profile.wallet_age_days = 45;
        profile.archetype_consistency = 0.90;
        profile.duration_variance = 0.15;
        let (kind, scores) = deterministic_dog(&profile);
        let q_avg = (scores.fidelity
            + scores.phi
            + scores.verify
            + scores.culture
            + scores.burn
            + scores.sovereignty)
            / 6.0;
        assert!(q_avg >= 0.54);
        assert!(kind == DeterministicDogKind::Howl || kind == DeterministicDogKind::Wag);
    }
}
