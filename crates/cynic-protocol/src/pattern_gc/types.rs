//! Pure types for generational pattern garbage collection.
//!
//! No I/O lives here: these shapes are consumed by pure reconciliation
//! (`policy::classify`) and produced by observation adapters in higher layers.

use serde::{Deserialize, Serialize};

/// Generational class of a recurring/emerging pattern.
///
/// `Fading` is a promotion signal, not a failure: a previously recurring
/// pattern whose raw frequency collapses is usually one whose knowledge was
/// successfully distilled into a durable artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternClass {
    /// Absolute recurrence proven over the full window.
    Recurring,
    /// Recent rate exceeds historical rate beyond epsilon with enough mass.
    Emerging,
    /// Formerly recurring pattern whose recent activity collapsed.
    Fading,
    /// Insufficient or contradictory signal; preserved as explicit unknown.
    Unknown,
}

/// Observation counts for one pattern key, split into two disjoint windows.
///
/// Counts alone are meaningless without their spans: 900 hits in a day and
/// 900 hits in six months are opposite signals. Both spans must be expressed
/// in the same unit (seconds recommended); the unit itself stays
/// adapter-owned because classification only consumes ratios.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternWindows {
    /// Count over the long (cold) window.
    pub historical: u64,
    /// Count over the short (hot) window immediately following it.
    pub recent: u64,
    /// Length of the historical window, same unit as `recent_span`.
    pub historical_span: u64,
    /// Length of the recent window, same unit as `historical_span`.
    pub recent_span: u64,
}

impl PatternWindows {
    /// Comparable windows of equal length (span `1` in the unit-agnostic
    /// sense). Useful when an adapter can only guarantee equal slices.
    pub fn comparable(historical: u64, recent: u64) -> Self {
        Self {
            historical,
            recent,
            historical_span: 1,
            recent_span: 1,
        }
    }
}

/// One observed pattern keyed by its canonical bucket name
/// (e.g. `error_buckets.toolchain_ice`, `tool_bigram.Bash->Read`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternObservation {
    /// Canonical, dot-separated pattern key.
    pub key: String,
    /// Windowed counts observed for this key.
    pub windows: PatternWindows,
}

/// Typed retention policy driving the classification decision.
///
/// Defaults are calibrated against the mined corpus
/// (`trace_report.json`, 2026-08-18): `toolchain_ice=5025` and
/// `network=1983` must clear [`RetentionPolicy::recurring_threshold_abs`]
/// while `oom=224` must remain below it and therefore stay unproven.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Absolute historical count above which a pattern is proven recurring.
    pub recurring_threshold_abs: u64,
    /// Minimum recent count required before an emerging claim is allowed.
    pub emergence_floor_abs: u64,
    /// Ratio margin `recent_rate/historical_rate - 1` required to call a
    /// rise emerging.
    pub emergence_ratio_epsilon: f64,
    /// A recurring pattern counts as fading once its recent rate falls
    /// below this fraction of its historical rate.
    pub fading_ratio_ceiling: f64,
}

impl RetentionPolicy {
    /// Policy calibrated on the observed trace corpus (see module docs).
    pub fn observed_default() -> Self {
        Self {
            recurring_threshold_abs: 500,
            emergence_floor_abs: 50,
            emergence_ratio_epsilon: 0.25,
            fading_ratio_ceiling: 0.5,
        }
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::observed_default()
    }
}

/// Classification outcome for a single pattern key.
///
/// Mirrors `ModelRetentionAssessment`: the assessment never authorizes the
/// mutation it describes. Raw-trace eviction is a bounded, consented action
/// owned by an apply surface; this contract records the class and reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternGcAssessment {
    pub schema: String,
    /// Canonical pattern key that was classified.
    pub key: String,
    pub class: PatternClass,
    /// Explicit human-readable grounds citing observed counts.
    pub reasons: Vec<String>,
    /// Always `false` at contract level; eviction requires an explicit,
    /// consented apply surface and read-back proof upstream.
    pub eviction_authorized: bool,
}
