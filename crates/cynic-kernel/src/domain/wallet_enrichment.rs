//! Compatibility adapter for wallet behavioral intelligence.
//!
//! Temporal wallet scoring lives in `cynic-intel`; this module keeps the current
//! kernel profile shape and maps scores into kernel dog verdict types.

use serde::{Deserialize, Serialize};

use crate::domain::dog::{AxiomScores, VerdictKind};
use cynic_intel::wallet_enrichment::{
    BehavioralVerdictKind, WalletBehaviorView, behavioral_dog as score_wallet_behavior,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub chess_profile: Option<super::wallet_judgment::WalletProfile>,
}

pub use cynic_intel::wallet_enrichment::{MIN_SWAPS_FOR_ANALYSIS, MIN_VERDICT_COVERAGE};

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

pub fn behavioral_dog(profile: &WalletBehavioralProfile) -> (VerdictKind, AxiomScores) {
    let (kind, scores) = score_wallet_behavior(profile);
    (
        match kind {
            BehavioralVerdictKind::Howl => VerdictKind::Howl,
            BehavioralVerdictKind::Wag => VerdictKind::Wag,
            BehavioralVerdictKind::Growl => VerdictKind::Growl,
            BehavioralVerdictKind::Bark => VerdictKind::Bark,
            BehavioralVerdictKind::Epoche => VerdictKind::Epoche,
        },
        AxiomScores {
            fidelity: scores.fidelity,
            phi: scores.phi,
            verify: scores.verify,
            culture: scores.culture,
            burn: scores.burn,
            sovereignty: scores.sovereignty,
            reasoning: Default::default(),
            ..Default::default()
        },
    )
}
