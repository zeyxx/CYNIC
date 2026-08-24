//! Pure reconciliation for pattern garbage collection.
//!
//! `classify` is the single canonical decision source (structural invariant
//! 6): CLI, MCP, and future apply surfaces must project from it, never
//! recompute classification locally. It consumes observed windowed counts
//! and the typed retention policy; it performs no I/O and never authorizes
//! eviction itself.

use super::types::{
    PatternClass, PatternGcAssessment, PatternObservation, PatternWindows, RetentionPolicy,
};

/// Classify one observed pattern from its windowed counts and spans.
///
/// Decision order is deliberate:
/// 1. no signal on either window -> [`PatternClass::Unknown`];
/// 2. historical mass below the recurrence threshold -> the only possible
///    claim is emergence (floor + rate ratio beyond epsilon), otherwise
///    unknown;
/// 3. proven recurrence -> emerging if the recent *rate* rises beyond
///    epsilon against the historical *rate*, fading once it collapses under
///    the policy's fading ceiling, otherwise stable-recurring.
///
/// All rise/collapse comparisons consume rates (count divided by span), so
/// verdicts are invariant to window-length choices. Zero or negative spans
/// make rates undefined: the observation is reported unknown rather than
/// guessed.
pub fn classify(observation: &PatternObservation, policy: &RetentionPolicy) -> PatternGcAssessment {
    let windows = &observation.windows;
    let mut reasons: Vec<String> = Vec::new();

    let class = match rates(windows) {
        None => {
            reasons.push("window spans missing or non-positive; rates undefined".to_string());
            PatternClass::Unknown
        }
        Some((historical_rate, recent_rate)) => {
            if windows.historical == 0 && windows.recent == 0 {
                reasons.push("no observations in either window".to_string());
                PatternClass::Unknown
            } else if windows.historical < policy.recurring_threshold_abs {
                if windows.recent >= policy.emergence_floor_abs
                    && recent_rate > historical_rate * (1.0 + policy.emergence_ratio_epsilon)
                {
                    reasons.push(format!(
                        "recent count {} exceeds floor {} with rate rise beyond epsilon",
                        windows.recent, policy.emergence_floor_abs
                    ));
                    PatternClass::Emerging
                } else {
                    reasons.push(format!(
                        "historical {} below recurring threshold {}; insufficient emergence signal",
                        windows.historical, policy.recurring_threshold_abs
                    ));
                    PatternClass::Unknown
                }
            } else if recent_rate > historical_rate * (1.0 + policy.emergence_ratio_epsilon) {
                reasons.push(format!(
                    "recurring mass {} with recent rate surge beyond epsilon",
                    windows.historical
                ));
                PatternClass::Emerging
            } else if recent_rate < historical_rate * policy.fading_ratio_ceiling {
                reasons.push(format!(
                    "former recurring pattern ({}) whose recent rate collapsed",
                    windows.historical
                ));
                PatternClass::Fading
            } else {
                reasons.push(format!(
                    "absolute recurrence {} with stable recent rate",
                    windows.historical
                ));
                PatternClass::Recurring
            }
        }
    };

    PatternGcAssessment {
        schema: SCHEMA.to_string(),
        key: observation.key.clone(),
        class,
        reasons,
        eviction_authorized: false,
    }
}

/// Classify a batch, preserving observation order.
pub fn classify_all(
    observations: &[PatternObservation],
    policy: &RetentionPolicy,
) -> Vec<PatternGcAssessment> {
    observations
        .iter()
        .map(|observation| classify(observation, policy))
        .collect()
}

/// Schema of every assessment produced by this contract.
pub const SCHEMA: &str = "cynic.pattern_gc.assessment.v1";

/// Per-unit rates for both windows.
///
/// A *nonzero* count with a zero span is contradictory input -> `None`
/// (explicit unknown). A zero count means zero rate regardless of span: a
/// key with no cold history is young, not undefined.
fn rates(windows: &PatternWindows) -> Option<(f64, f64)> {
    if windows.historical > 0 && windows.historical_span == 0 {
        return None;
    }
    if windows.recent > 0 && windows.recent_span == 0 {
        return None;
    }
    let historical_rate = if windows.historical == 0 {
        0.0
    } else {
        windows.historical as f64 / windows.historical_span as f64
    };
    let recent_rate = if windows.recent == 0 {
        0.0
    } else {
        windows.recent as f64 / windows.recent_span as f64
    };
    Some((historical_rate, recent_rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(key: &str, historical: u64, recent: u64) -> PatternObservation {
        PatternObservation {
            key: key.to_string(),
            windows: PatternWindows::comparable(historical, recent),
        }
    }

    fn policy() -> RetentionPolicy {
        RetentionPolicy::observed_default()
    }

    #[test]
    fn corpus_calibrated_defaults_place_known_buckets() {
        let p = policy();
        // Real mined corpus evidence (trace_report.json, 2026-08-18).
        assert!(5025 >= p.recurring_threshold_abs); // toolchain_ice proven
        assert!(1983 >= p.recurring_threshold_abs); // network proven
        assert!(224 < p.recurring_threshold_abs); // oom stays unproven
    }

    #[test]
    fn empty_windows_are_unknown_not_recurring() {
        let a = classify(&obs("x", 0, 0), &policy());
        assert_eq!(a.class, PatternClass::Unknown);
        assert!(!a.eviction_authorized);
    }

    #[test]
    fn zero_spans_are_explicit_unknown() {
        let mut o = obs("k", 100, 10);
        o.windows.recent_span = 0;
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Unknown);
        assert!(a.reasons[0].contains("spans"));
    }

    #[test]
    fn young_key_with_zero_cold_span_classifies_not_unknown() {
        // A key with NO cold history is young, not undefined: zero events
        // on any span means zero cold rate. Regression proof for the
        // scan-adapter integration catch (2026-08-24).
        let o = PatternObservation {
            key: "tool.brand_new".to_string(),
            windows: PatternWindows {
                historical: 0,
                recent: 60,
                historical_span: 0,
                recent_span: 22,
            },
        };
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Emerging);
        // Contradictory input stays rejected: nonzero count, zero span.
        let mut bad = obs("k", 100, 10);
        bad.windows.historical_span = 0;
        assert_eq!(classify(&bad, &policy()).class, PatternClass::Unknown);
    }

    #[test]
    fn high_absolute_mass_with_stable_rate_is_recurring() {
        // 5025 over ~10 months cold, 900 in the last 2 months: steady pace.
        let mut o = obs("error_buckets.toolchain_ice", 5025, 900);
        o.windows.historical_span = 300;
        o.windows.recent_span = 60;
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Recurring);
    }

    #[test]
    fn low_mass_with_surge_is_emerging() {
        // ts_exec style: new tool rising sharply on one project.
        let mut o = obs("tool.ts_exec", 120, 400);
        o.windows.historical_span = 60;
        o.windows.recent_span = 15;
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Emerging);
    }

    #[test]
    fn low_mass_without_floor_or_ratio_stays_unknown() {
        let o = obs("tool.rare", 40, 3);
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Unknown);
    }

    #[test]
    fn recent_only_history_with_floor_is_emerging() {
        let o = obs("tool.brand_new", 0, 300);
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Emerging);
    }

    #[test]
    fn collapsing_former_pattern_is_fading() {
        // Distillation success signature: was huge, now nearly silent.
        let mut o = obs("error.cargo_build", 5000, 20);
        o.windows.historical_span = 500;
        o.windows.recent_span = 50;
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Fading);
    }

    #[test]
    fn steady_recurrence_at_realistic_spans_is_recurring() {
        // network errors: ~1983 over a year cold, 180 over the last month.
        let mut o = obs("error.network", 1983, 180);
        o.windows.historical_span = 365;
        o.windows.recent_span = 30;
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Recurring);
    }

    #[test]
    fn equal_spans_expose_true_collapse_as_fading() {
        // Equal windows mean raw comparison applies: 180 vs 1983 IS a
        // collapse, and the contract must say so instead of flattering us.
        let o = obs("error.network", 1983, 180);
        let a = classify(&o, &policy());
        assert_eq!(a.class, PatternClass::Fading);
    }

    #[test]
    fn same_verdict_regardless_of_window_unit_choice() {
        // Same shape expressed in days vs hours: identical classification.
        let mut days = obs("k", 600, 30);
        days.windows.historical_span = 30;
        days.windows.recent_span = 5;

        let mut hours = obs("k", 600, 30);
        hours.windows.historical_span = 720;
        hours.windows.recent_span = 120;

        let p = policy();
        assert_eq!(classify(&days, &p).class, classify(&hours, &p).class);
    }

    #[test]
    fn batch_preserves_order_and_schema() {
        let assessments = classify_all(&[obs("a", 600, 100), obs("b", 5, 1)], &policy());
        assert_eq!(assessments.len(), 2);
        assert_eq!(assessments[0].key, "a");
        assert!(assessments.iter().all(|a| a.schema == SCHEMA));
        assert!(assessments.iter().all(|a| !a.eviction_authorized));
    }
}
