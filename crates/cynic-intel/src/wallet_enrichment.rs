use crate::token_postprocessor::AxiomScoreVector;

pub const MIN_SWAPS_FOR_ANALYSIS: u32 = 3;
pub const MIN_VERDICT_COVERAGE: f64 = 0.1;
const PHI_INV: f64 = 0.618_033_988_749_894_8;

pub trait WalletBehaviorView {
    fn wallet_age_days(&self) -> u32;
    fn total_swaps(&self) -> u32;
    fn swaps_per_day(&self) -> f64;
    fn distinct_tokens_traded(&self) -> u32;
    fn pump_fun_ratio(&self) -> f64;
    fn median_hold_hours(&self) -> f64;
    fn flip_count(&self) -> u32;
    fn total_volume_sol(&self) -> f64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehavioralVerdictKind {
    Howl,
    Wag,
    Growl,
    Bark,
    Epoche,
}

pub fn behavioral_dog(
    profile: &impl WalletBehaviorView,
) -> (BehavioralVerdictKind, AxiomScoreVector) {
    if profile.total_swaps() < MIN_SWAPS_FOR_ANALYSIS && profile.wallet_age_days() < 7 {
        return (
            BehavioralVerdictKind::Epoche,
            AxiomScoreVector {
                fidelity: 0.0,
                phi: 0.0,
                verify: 0.0,
                culture: 0.0,
                burn: 0.0,
                sovereignty: 0.0,
            },
        );
    }

    let fidelity = if profile.flip_count() == 0 && profile.wallet_age_days() >= 30 {
        0.55
    } else if profile.flip_count() <= 2 && profile.median_hold_hours() > 48.0 {
        0.45
    } else if profile.flip_count() > 5 && profile.median_hold_hours() < 1.0 {
        0.10
    } else if profile.flip_count() > 3 {
        0.20
    } else {
        0.35
    };

    let phi = if profile.swaps_per_day() > 20.0 {
        0.10
    } else if profile.swaps_per_day() > 5.0 {
        0.20
    } else if profile.swaps_per_day() > 0.0 && profile.swaps_per_day() <= 2.0 {
        0.50
    } else if profile.total_swaps() == 0 && profile.wallet_age_days() >= 30 {
        0.55
    } else {
        0.35
    };

    let verify = if profile.total_swaps() >= 10 {
        0.55
    } else if profile.total_swaps() >= MIN_SWAPS_FOR_ANALYSIS {
        0.45
    } else {
        0.35
    };

    let culture = if profile.pump_fun_ratio() > 0.9 && profile.total_swaps() > 10 {
        0.15
    } else if profile.pump_fun_ratio() > 0.7 {
        0.25
    } else if profile.pump_fun_ratio() < 0.3 && profile.distinct_tokens_traded() >= 3 {
        0.50
    } else if profile.total_swaps() == 0 && profile.wallet_age_days() >= 30 {
        0.45
    } else {
        0.35
    };

    let burn = if profile.total_volume_sol() > 1.0 && profile.total_swaps() <= 10 {
        0.50
    } else if profile.total_swaps() > 20 && profile.total_volume_sol() < 0.5 {
        0.15
    } else if profile.total_swaps() == 0 {
        0.55
    } else {
        0.35
    };

    let sovereignty = if profile.swaps_per_day() > 10.0 && profile.pump_fun_ratio() > 0.8 {
        0.10
    } else if profile.wallet_age_days() >= 30 && profile.flip_count() <= 1 {
        0.55
    } else if profile.distinct_tokens_traded() >= 5 && profile.pump_fun_ratio() < 0.5 {
        0.45
    } else {
        0.30
    };

    let q_score: f64 = (fidelity + phi + verify + culture + burn + sovereignty) / 6.0;
    let q_capped = if profile.wallet_age_days() < 3 {
        q_score.min(0.25)
    } else if profile.wallet_age_days() < 7 {
        q_score.min(0.35)
    } else if profile.wallet_age_days() < 30 {
        q_score.min(0.50)
    } else {
        q_score
    };

    let kind = if q_capped >= PHI_INV {
        BehavioralVerdictKind::Howl
    } else if q_capped >= 0.382 {
        BehavioralVerdictKind::Wag
    } else if q_capped >= 0.236 {
        BehavioralVerdictKind::Growl
    } else {
        BehavioralVerdictKind::Bark
    };

    (
        kind,
        AxiomScoreVector {
            fidelity,
            phi,
            verify,
            culture,
            burn,
            sovereignty,
        },
    )
}

/// Behavioral profile assembled from on-chain history (Helius enrichment).
/// Canonical owner: `cynic-intel` (SoC foundation §481). Consumers import it
/// from here, never from a kernel-local copy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalletBehavioralProfile {
    pub wallet_address: String,
    pub wallet_age_days: u32,
    pub sol_balance: f64,
    pub total_swaps: u32,
    pub swaps_per_day: f64,
    pub distinct_tokens_traded: u32,
    pub pump_fun_ratio: f64,
    pub median_hold_hours: f64,
    pub flip_count: u32,
    pub long_hold_count: u32,
    pub total_volume_sol: f64,
    pub fungible_token_count: u32,
    pub chess_profile: Option<crate::wallet_judgment::WalletProfile>,
}

impl WalletBehaviorView for WalletBehavioralProfile {
    fn wallet_age_days(&self) -> u32 {
        self.wallet_age_days
    }

    fn total_swaps(&self) -> u32 {
        self.total_swaps
    }

    fn swaps_per_day(&self) -> f64 {
        self.swaps_per_day
    }

    fn distinct_tokens_traded(&self) -> u32 {
        self.distinct_tokens_traded
    }

    fn pump_fun_ratio(&self) -> f64 {
        self.pump_fun_ratio
    }

    fn median_hold_hours(&self) -> f64 {
        self.median_hold_hours
    }

    fn flip_count(&self) -> u32 {
        self.flip_count
    }

    fn total_volume_sol(&self) -> f64 {
        self.total_volume_sol
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct FixtureProfile {
        wallet_age_days: u32,
        total_swaps: u32,
        swaps_per_day: f64,
        distinct_tokens_traded: u32,
        pump_fun_ratio: f64,
        median_hold_hours: f64,
        flip_count: u32,
        total_volume_sol: f64,
    }

    impl WalletBehaviorView for FixtureProfile {
        fn wallet_age_days(&self) -> u32 {
            self.wallet_age_days
        }

        fn total_swaps(&self) -> u32 {
            self.total_swaps
        }

        fn swaps_per_day(&self) -> f64 {
            self.swaps_per_day
        }

        fn distinct_tokens_traded(&self) -> u32 {
            self.distinct_tokens_traded
        }

        fn pump_fun_ratio(&self) -> f64 {
            self.pump_fun_ratio
        }

        fn median_hold_hours(&self) -> f64 {
            self.median_hold_hours
        }

        fn flip_count(&self) -> u32 {
            self.flip_count
        }

        fn total_volume_sol(&self) -> f64 {
            self.total_volume_sol
        }
    }

    fn diamond_hands_profile() -> FixtureProfile {
        FixtureProfile {
            wallet_age_days: 300,
            total_swaps: 0,
            swaps_per_day: 0.0,
            distinct_tokens_traded: 0,
            pump_fun_ratio: 0.0,
            median_hold_hours: 0.0,
            flip_count: 0,
            total_volume_sol: 0.0,
        }
    }

    fn flipper_profile() -> FixtureProfile {
        FixtureProfile {
            wallet_age_days: 5,
            total_swaps: 50,
            swaps_per_day: 25.0,
            distinct_tokens_traded: 20,
            pump_fun_ratio: 0.95,
            median_hold_hours: 0.1,
            flip_count: 15,
            total_volume_sol: 0.3,
        }
    }

    #[test]
    fn diamond_hands_scores_wag_or_howl() {
        let (kind, scores) = behavioral_dog(&diamond_hands_profile());
        assert!(kind == BehavioralVerdictKind::Wag || kind == BehavioralVerdictKind::Howl);
        assert!(scores.fidelity >= 0.5);
        assert!(scores.sovereignty >= 0.5);
    }

    #[test]
    fn flipper_scores_bark() {
        let (kind, scores) = behavioral_dog(&flipper_profile());
        assert_eq!(kind, BehavioralVerdictKind::Bark);
        assert!(scores.fidelity <= 0.20);
        assert!(scores.sovereignty <= 0.15);
    }

    #[test]
    fn new_wallet_few_swaps_epoche() {
        let mut profile = diamond_hands_profile();
        profile.wallet_age_days = 2;
        profile.total_swaps = 1;
        let (kind, _) = behavioral_dog(&profile);
        assert_eq!(kind, BehavioralVerdictKind::Epoche);
    }
}
