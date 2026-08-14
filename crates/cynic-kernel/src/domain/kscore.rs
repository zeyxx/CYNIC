//! Compatibility adapter for K-Score computation.
//! K-Score domain heuristics now live in `cynic-intel`.

use crate::domain::enrichment::{HolderClass, KScore, WalletBehavior};
use cynic_intel::kscore::{self as kscore_intel, HolderClass as IntelHolderClass};

impl cynic_intel::kscore::KScoreConfigView for crate::infra::config::KScoreConfig {
    fn weight_diamond_hands(&self) -> f64 {
        self.weight_diamond_hands
    }

    fn weight_organic_growth(&self) -> f64 {
        self.weight_organic_growth
    }

    fn weight_longevity(&self) -> f64 {
        self.weight_longevity
    }

    fn accumulator_threshold(&self) -> f64 {
        self.accumulator_threshold
    }

    fn holder_threshold(&self) -> f64 {
        self.holder_threshold
    }

    fn reducer_threshold(&self) -> f64 {
        self.reducer_threshold
    }
}

struct WalletBehaviorAdapter<'a>(&'a WalletBehavior);

impl cynic_intel::kscore::WalletBehaviorView for WalletBehaviorAdapter<'_> {
    fn class(&self) -> kscore_intel::HolderClass {
        map_holder_class_to_intel(&self.0.class)
    }

    fn hold_time_hours(&self) -> Option<f64> {
        self.0.hold_time_hours
    }
}

fn map_holder_class_to_intel(class: &HolderClass) -> kscore_intel::HolderClass {
    match class {
        HolderClass::Accumulator => kscore_intel::HolderClass::Accumulator,
        HolderClass::Holder => kscore_intel::HolderClass::Holder,
        HolderClass::Reducer => kscore_intel::HolderClass::Reducer,
        HolderClass::Extractor => kscore_intel::HolderClass::Extractor,
    }
}

fn map_holder_class_from_intel(class: kscore_intel::HolderClass) -> HolderClass {
    match class {
        IntelHolderClass::Accumulator => HolderClass::Accumulator,
        IntelHolderClass::Holder => HolderClass::Holder,
        IntelHolderClass::Reducer => HolderClass::Reducer,
        IntelHolderClass::Extractor => HolderClass::Extractor,
    }
}

/// Classify a wallet retention ratio using the intel-owned thresholds.
pub fn classify_wallet(retention: f64, config: &crate::infra::config::KScoreConfig) -> HolderClass {
    let intel_class = kscore_intel::classify_wallet(retention, config);
    map_holder_class_from_intel(intel_class)
}

/// Compute token K-Score with intel-owned mathematics.
pub fn compute_kscore(
    behaviors: &[WalletBehavior],
    holder_count: u64,
    top10_pct: f64,
    age_hours: u64,
    config: &crate::infra::config::KScoreConfig,
) -> KScore {
    let adapted: Vec<_> = behaviors.iter().map(WalletBehaviorAdapter).collect();
    let score = kscore_intel::compute_kscore(&adapted, holder_count, top10_pct, age_hours, config);
    KScore {
        score: score.score,
        diamond_hands: score.diamond_hands,
        organic_growth: score.organic_growth,
        longevity: score.longevity,
        wallets_analyzed: score.wallets_analyzed,
        accumulators: score.accumulators,
        holders: score.holders,
        reducers: score.reducers,
        extractors: score.extractors,
        median_hold_hours: score.median_hold_hours,
    }
}
