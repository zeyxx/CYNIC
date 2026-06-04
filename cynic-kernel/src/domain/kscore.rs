//! Pure K-Score computation — zero I/O, zero async.
//! Extracted from backends/helius.rs::analyze_behaviors().

use crate::domain::enrichment::{HolderClass, KScore, WalletBehavior};
use crate::infra::config::KScoreConfig;

/// Classify a wallet by its retention ratio.
pub fn classify_wallet(retention: f64, config: &KScoreConfig) -> HolderClass {
    if retention >= config.accumulator_threshold {
        HolderClass::Accumulator
    } else if retention >= config.holder_threshold {
        HolderClass::Holder
    } else if retention >= config.reducer_threshold {
        HolderClass::Reducer
    } else {
        HolderClass::Extractor
    }
}

/// Compute K-Score from behavioral data. Pure function.
///
/// # Arguments
/// * `behaviors`    — per-wallet behavioral classification (already built by analyze_behaviors)
/// * `holder_count` — total number of unique token holders (from on-chain data)
/// * `top10_pct`    — percentage held by the top 10 wallets (0.0–100.0)
/// * `age_hours`    — token age in hours since creation
/// * `config`       — K-Score weights and thresholds
pub fn compute_kscore(
    behaviors: &[WalletBehavior],
    holder_count: u64,
    top10_pct: f64,
    age_hours: u64,
    config: &KScoreConfig,
) -> KScore {
    let total = behaviors.len() as f64;
    if total == 0.0 {
        return KScore::default();
    }

    let acc = behaviors
        .iter()
        .filter(|b| b.class == HolderClass::Accumulator)
        .count() as f64;
    let hld = behaviors
        .iter()
        .filter(|b| b.class == HolderClass::Holder)
        .count() as f64;
    let ext = behaviors
        .iter()
        .filter(|b| b.class == HolderClass::Extractor)
        .count() as f64;
    let red = behaviors
        .iter()
        .filter(|b| b.class == HolderClass::Reducer)
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
    let score = diamond_hands.powf(config.weight_diamond_hands)
        * organic_growth.powf(config.weight_organic_growth)
        * longevity.powf(config.weight_longevity);

    // Median hold time across all analyzed wallets for this token.
    let mut hold_times: Vec<f64> = behaviors.iter().filter_map(|b| b.hold_time_hours).collect();
    hold_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_hold_hours = if hold_times.is_empty() {
        None
    } else {
        let mid = hold_times.len() / 2;
        if hold_times.len() % 2 == 0 && hold_times.len() >= 2 {
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

    fn cfg() -> KScoreConfig {
        KScoreConfig::default()
    }

    #[test]
    fn empty_behaviors_returns_zero_score() {
        let score = compute_kscore(&[], 100, 30.0, 200, &cfg());
        assert_eq!(score.score, 0.0);
        assert_eq!(score.wallets_analyzed, 0);
    }

    #[test]
    fn all_accumulators_produce_high_diamond_hands() {
        let behaviors: Vec<WalletBehavior> = (0..5)
            .map(|_| WalletBehavior {
                class: HolderClass::Accumulator,
                retention_ratio: 2.0,
                swap_count: 3,
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
        let behaviors: Vec<WalletBehavior> = (0..5)
            .map(|_| WalletBehavior {
                class: HolderClass::Extractor,
                retention_ratio: 0.1,
                swap_count: 5,
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
        let b = vec![WalletBehavior {
            class: HolderClass::Holder,
            retention_ratio: 1.0,
            swap_count: 1,
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
            WalletBehavior {
                class: HolderClass::Holder,
                retention_ratio: 1.1,
                swap_count: 2,
                hold_time_hours: None,
            },
            WalletBehavior {
                class: HolderClass::Reducer,
                retention_ratio: 0.6,
                swap_count: 3,
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
            WalletBehavior {
                class: HolderClass::Holder,
                retention_ratio: 1.0,
                swap_count: 2,
                hold_time_hours: Some(48.0), // 2 days
            },
            WalletBehavior {
                class: HolderClass::Holder,
                retention_ratio: 1.0,
                swap_count: 1,
                hold_time_hours: Some(240.0), // 10 days
            },
            WalletBehavior {
                class: HolderClass::Reducer,
                retention_ratio: 0.7,
                swap_count: 3,
                hold_time_hours: None, // no buy found
            },
        ];
        let score = compute_kscore(&behaviors, 500, 30.0, 200, &cfg());
        // Median of [48.0, 240.0] (None excluded) = (48+240)/2 = 144.0
        assert_eq!(score.median_hold_hours, Some(144.0));
    }

    #[test]
    fn median_hold_hours_none_when_no_data() {
        let behaviors = vec![WalletBehavior {
            class: HolderClass::Holder,
            retention_ratio: 1.0,
            swap_count: 1,
            hold_time_hours: None,
        }];
        let score = compute_kscore(&behaviors, 100, 30.0, 200, &cfg());
        assert_eq!(score.median_hold_hours, None);
    }
}
