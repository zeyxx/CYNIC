//! Machine-readable pattern GC report with content digest proof.
//!
//! The report is the read-back surface of this contract: it aggregates
//! classifications under an explicit policy and boundary snapshot, and
//! seals itself with a SHA-256 over its canonical JSON so any consumer can
//! detect drift between what was classified and what is being projected.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::policy::SCHEMA as ASSESSMENT_SCHEMA;
use super::types::{RateTrend, RateVerdict, Thresholds};

pub const REPORT_SCHEMA: &str = "cynic.rate_classify.report.v1";

/// Per-class tally over one classification run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassTally {
    pub recurring: u64,
    pub emerging: u64,
    pub fading: u64,
    pub unknown: u64,
}

impl ClassTally {
    fn from_assessments(assessments: &[RateVerdict]) -> Self {
        let mut tally = Self::default();
        for assessment in assessments {
            match assessment.class {
                RateTrend::Recurring => tally.recurring += 1,
                RateTrend::Emerging => tally.emerging += 1,
                RateTrend::Fading => tally.fading += 1,
                RateTrend::Unknown => tally.unknown += 1,
            }
        }
        tally
    }
}

/// Aggregated, self-sealing classification report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateReport {
    pub schema: String,
    /// Assessment schema this report was built from.
    pub assessment_schema: String,
    /// Policy snapshot that produced every assessment (explicit desired
    /// state; no hidden defaults).
    pub policy: Thresholds,
    /// Windowing parameters as declared by the caller (unix seconds).
    pub boundary_s: i64,
    pub recent_span_s: u64,
    /// Number of observations whose events were stamped beyond the
    /// boundary (from `scan`, surfaced verbatim).
    pub out_of_order_events: u64,
    pub tally: ClassTally,
    /// Canonical keys in original observation order.
    pub keys: Vec<String>,
    /// SHA-256 hex digest over the canonical serialization of
    /// `(policy, boundary_s, recent_span_s, assessments)`.
    pub content_sha256: String,
}

/// Build the report from assessments plus the exact inputs that produced
/// them. Pure: same inputs always yield the same digest.
pub fn build_report(
    assessments: &[RateVerdict],
    policy: &Thresholds,
    boundary_s: i64,
    recent_span_s: u64,
    out_of_order_events: u64,
) -> RateReport {
    let mut hasher = Sha256::new();
    // Deterministic field order: serde_json of a fixed tuple shape.
    let digest_input = (policy, boundary_s, recent_span_s, assessments);
    hasher.update(serde_json::to_vec(&digest_input).unwrap_or_default());
    let content_sha256 = hex::encode(hasher.finalize());

    RateReport {
        schema: REPORT_SCHEMA.to_string(),
        assessment_schema: ASSESSMENT_SCHEMA.to_string(),
        policy: policy.clone(),
        boundary_s,
        recent_span_s,
        out_of_order_events,
        tally: ClassTally::from_assessments(assessments),
        keys: assessments.iter().map(|a| a.key.clone()).collect(),
        content_sha256,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_classify::types::{WindowedCount, CountWindows};

    fn obs(key: &str, historical: u64, recent: u64) -> WindowedCount {
        WindowedCount {
            key: key.to_string(),
            windows: CountWindows::comparable(historical, recent),
        }
    }

    fn sample() -> Vec<RateVerdict> {
        let p = Thresholds::observed_default();
        let mut ice = obs("error_buckets.toolchain_ice", 5025, 900);
        // Realistic spans: steady pace stays recurring (comparable spans
        // would read as collapse — see policy tests).
        ice.windows.historical_span = 300;
        ice.windows.recent_span = 60;
        crate::rate_classify::classify_all(
            &[ice, obs("tool.ts_exec", 120, 400), obs("oom", 224, 30)],
            &p,
        )
    }

    #[test]
    fn tally_counts_every_class() {
        let p = Thresholds::observed_default();
        let r = build_report(&sample(), &p, 1_000_000, 100, 0);
        // ice=Recurring, ts_exec=Emerging, oom=Unknown at comparable spans.
        assert_eq!(r.tally.recurring, 1);
        assert_eq!(r.tally.emerging, 1);
        assert_eq!(r.tally.unknown, 1);
        assert_eq!(r.tally.fading, 0);
        assert_eq!(
            r.keys,
            vec![
                "error_buckets.toolchain_ice".to_string(),
                "tool.ts_exec".to_string(),
                "oom".to_string()
            ]
        );
    }

    #[test]
    fn digest_is_deterministic_and_input_sensitive() {
        let p = Thresholds::observed_default();
        let a = sample();
        let r1 = build_report(&a, &p, 1_000_000, 100, 0);
        let r2 = build_report(&a, &p, 1_000_000, 100, 0);
        assert_eq!(r1.content_sha256, r2.content_sha256);

        let r3 = build_report(&a, &p, 1_000_001, 100, 0);
        assert_ne!(r1.content_sha256, r3.content_sha256);

        let mut a4 = sample();
        a4[0].class = RateTrend::Fading;
        let r4 = build_report(&a4, &p, 1_000_000, 100, 0);
        assert_ne!(r1.content_sha256, r4.content_sha256);
    }

    #[test]
    fn report_roundtrips_through_json_with_schema_labels() {
        let p = Thresholds::observed_default();
        let r = build_report(&sample(), &p, 1_000_000, 100, 7);
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(REPORT_SCHEMA));
        assert!(json.contains("content_sha256"));
        let back: RateReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
        assert_eq!(back.out_of_order_events, 7);
    }

    #[test]
    fn empty_run_yields_zero_tally_not_error() {
        let p = Thresholds::observed_default();
        let r = build_report(&[], &p, 0, 100, 0);
        assert_eq!(r.tally, ClassTally::default());
        assert!(r.keys.is_empty());
    }
}
