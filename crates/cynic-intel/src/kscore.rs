//! Pure K-Score computation and classification for token wallet behaviors.
//!
//! This crate owns the behavioral intelligence math. The kernel adapts its local
//! config and behavior shapes through thin traits.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HolderClass {
    Accumulator,
    Holder,
    Reducer,
    Extractor,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct KScore {
    pub score: f64,
    pub diamond_hands: f64,
    pub organic_growth: f64,
    pub longevity: f64,
    pub wallets_analyzed: u32,
    pub accumulators: u32,
    pub holders: u32,
    pub reducers: u32,
    pub extractors: u32,
    pub median_hold_hours: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KScoreConfigSnapshot {
    pub enabled: bool,
    pub top_n_wallets: usize,
    pub swap_history_limit: usize,
    pub weight_diamond_hands: f64,
    pub weight_organic_growth: f64,
    pub weight_longevity: f64,
    pub accumulator_threshold: f64,
    pub holder_threshold: f64,
    pub reducer_threshold: f64,
}

impl Default for KScoreConfigSnapshot {
    fn default() -> Self {
        Self {
            enabled: true,
            top_n_wallets: 10,
            swap_history_limit: 20,
            weight_diamond_hands: 0.40,
            weight_organic_growth: 0.35,
            weight_longevity: 0.25,
            accumulator_threshold: 0.80,
            holder_threshold: 0.50,
            reducer_threshold: 0.20,
        }
    }
}

impl KScoreConfigView for KScoreConfigSnapshot {
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

pub trait KScoreConfigView {
    fn weight_diamond_hands(&self) -> f64;
    fn weight_organic_growth(&self) -> f64;
    fn weight_longevity(&self) -> f64;
    fn accumulator_threshold(&self) -> f64;
    fn holder_threshold(&self) -> f64;
    fn reducer_threshold(&self) -> f64;
}

pub trait WalletBehaviorView {
    fn class(&self) -> HolderClass;
    fn hold_time_hours(&self) -> Option<f64>;
}

/// Classify wallet behavior from retention ratio and thresholds.
pub fn classify_wallet(retention: f64, config: &impl KScoreConfigView) -> HolderClass {
    if retention >= config.accumulator_threshold() {
        HolderClass::Accumulator
    } else if retention >= config.holder_threshold() {
        HolderClass::Holder
    } else if retention >= config.reducer_threshold() {
        HolderClass::Reducer
    } else {
        HolderClass::Extractor
    }
}

/// Compute a K-Score from behavioral classifications.
///
/// Returns a deterministic score bundle, with no I/O and no async.
pub fn compute_kscore<B: WalletBehaviorView>(
    behaviors: &[B],
    holder_count: u64,
    top10_pct: f64,
    age_hours: u64,
    config: &impl KScoreConfigView,
) -> KScore {
    let total = behaviors.len() as f64;
    if total == 0.0 {
        return KScore::default();
    }

    let acc = behaviors
        .iter()
        .filter(|b| b.class() == HolderClass::Accumulator)
        .count() as f64;
    let hld = behaviors
        .iter()
        .filter(|b| b.class() == HolderClass::Holder)
        .count() as f64;
    let ext = behaviors
        .iter()
        .filter(|b| b.class() == HolderClass::Extractor)
        .count() as f64;
    let red = behaviors
        .iter()
        .filter(|b| b.class() == HolderClass::Reducer)
        .count() as f64;

    // DiamondHands = sqrt(conviction × retention_signal)
    // conviction: fraction of analyzed wallets that are accumulating or holding
    // retention_signal: tanh(acc/ext ratio / 2)
    //   tanh maps [0,∞) → [0,1).
    //   When acc >> ext: tanh → 1.0 (strong diamond hands signal).
    //   When ext >> acc: tanh → 0.0 (everyone selling).
    //   Division by 2 normalizes so 1:1 ratio → tanh(0.5) ≈ 0.46 (neutral).
    let conviction = (acc + hld) / total;
    let retention_signal = (acc / ext.max(1.0) / 2.0).tanh();
    let diamond_hands = (conviction * retention_signal).sqrt();

    // OrganicGrowth = sqrt(holder_norm × inv_concentration)
    let holder_norm = 1.0 - 1.0 / (1.0 + (1.0 + holder_count as f64 / 100.0).ln());
    let inv_concentration = (1.0 - top10_pct / 100.0).max(0.0);
    let organic_growth = (holder_norm * inv_concentration).sqrt();

    // Longevity = 1 - e^(-age_days/21)
    let age_days = age_hours as f64 / 24.0;
    let longevity = 1.0 - (-age_days / 21.0).exp();

    // K = DH^w1 × OG^w2 × L^w3
    let score = diamond_hands.powf(config.weight_diamond_hands())
        * organic_growth.powf(config.weight_organic_growth())
        * longevity.powf(config.weight_longevity());

    // Median hold time across all analyzed wallets for this token.
    let mut hold_times: Vec<f64> = behaviors
        .iter()
        .filter_map(|b| b.hold_time_hours())
        .collect();
    hold_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_hold_hours = if hold_times.is_empty() {
        None
    } else {
        let mid = hold_times.len() / 2;
        if hold_times.len().is_multiple_of(2) && hold_times.len() >= 2 {
            Some((hold_times[mid - 1] + hold_times[mid]) / 2.0)
        } else {
            Some(hold_times[mid])
        }
    };

    KScore {
        score,
        diamond_hands,
        organic_growth,
        longevity,
        wallets_analyzed: total as u32,
        accumulators: acc as u32,
        holders: hld as u32,
        reducers: red as u32,
        extractors: ext as u32,
        median_hold_hours,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct FixtureBehavior {
        class: HolderClass,
        hold_time_hours: Option<f64>,
    }

    impl WalletBehaviorView for FixtureBehavior {
        fn class(&self) -> HolderClass {
            self.class
        }

        fn hold_time_hours(&self) -> Option<f64> {
            self.hold_time_hours
        }
    }

    fn cfg() -> KScoreConfigSnapshot {
        KScoreConfigSnapshot {
            enabled: true,
            top_n_wallets: 10,
            swap_history_limit: 20,
            weight_diamond_hands: 0.75,
            weight_organic_growth: 1.0,
            weight_longevity: 1.0,
            accumulator_threshold: 1.5,
            holder_threshold: 1.0,
            reducer_threshold: 0.5,
        }
    }

    #[test]
    fn empty_behaviors_returns_zero_score() {
        let behaviors: Vec<FixtureBehavior> = Vec::new();
        let score = compute_kscore(&behaviors, 100, 30.0, 200, &cfg());
        assert_eq!(score.score, 0.0);
        assert_eq!(score.wallets_analyzed, 0);
    }

    #[test]
    fn all_accumulators_produce_high_diamond_hands() {
        let behaviors: Vec<_> = (0..5)
            .map(|_| FixtureBehavior {
                class: HolderClass::Accumulator,
                hold_time_hours: None,
            })
            .collect();
        let score = compute_kscore(&behaviors, 5000, 20.0, 720, &cfg());
        assert!(
            score.diamond_hands > 0.5,
            "expected DH > 0.5, got {}",
            score.diamond_hands
        );
        assert_eq!(score.accumulators, 5);
        assert_eq!(score.extractors, 0);
    }

    #[test]
    fn all_extractors_produce_low_diamond_hands() {
        let behaviors: Vec<_> = (0..5)
            .map(|_| FixtureBehavior {
                class: HolderClass::Extractor,
                hold_time_hours: None,
            })
            .collect();
        let score = compute_kscore(&behaviors, 1000, 80.0, 48, &cfg());
        assert!(
            score.diamond_hands < 0.2,
            "expected low DH, got {}",
            score.diamond_hands
        );
    }

    #[test]
    fn classify_wallet_respects_thresholds() {
        let c = cfg();
        assert_eq!(classify_wallet(2.0, &c), HolderClass::Accumulator);
        assert_eq!(classify_wallet(1.5, &c), HolderClass::Accumulator);
        assert_eq!(classify_wallet(1.0, &c), HolderClass::Holder);
        assert_eq!(classify_wallet(0.5, &c), HolderClass::Reducer);
        assert_eq!(classify_wallet(0.3, &c), HolderClass::Extractor);
    }

    #[test]
    fn longevity_increases_with_age() {
        let b = vec![FixtureBehavior {
            class: HolderClass::Holder,
            hold_time_hours: None,
        }];
        let s1h = compute_kscore(&b, 100, 30.0, 1, &cfg());
        let s30d = compute_kscore(&b, 100, 30.0, 720, &cfg());
        assert!(
            s1h.longevity < s30d.longevity,
            "longevity should increase with age"
        );
    }

    #[test]
    fn output_is_deterministic() {
        let b = vec![
            FixtureBehavior {
                class: HolderClass::Holder,
                hold_time_hours: None,
            },
            FixtureBehavior {
                class: HolderClass::Reducer,
                hold_time_hours: None,
            },
        ];
        let s1 = compute_kscore(&b, 1000, 40.0, 200, &cfg());
        let s2 = compute_kscore(&b, 1000, 40.0, 200, &cfg());
        assert_eq!(s1.score, s2.score);
    }

    #[test]
    fn median_hold_hours_computed_correctly() {
        let behaviors = vec![
            FixtureBehavior {
                class: HolderClass::Holder,
                hold_time_hours: Some(48.0),
            },
            FixtureBehavior {
                class: HolderClass::Holder,
                hold_time_hours: Some(240.0),
            },
            FixtureBehavior {
                class: HolderClass::Reducer,
                hold_time_hours: None,
            },
        ];
        let score = compute_kscore(&behaviors, 500, 30.0, 200, &cfg());
        assert_eq!(score.median_hold_hours, Some(144.0));
    }

    #[test]
    fn median_hold_hours_none_when_no_data() {
        let behaviors = vec![FixtureBehavior {
            class: HolderClass::Holder,
            hold_time_hours: None,
        }];
        let score = compute_kscore(&behaviors, 100, 30.0, 200, &cfg());
        assert_eq!(score.median_hold_hours, None);
    }
}
