//! Incremental ingestion contracts for the pattern GC (L1 observation layer).
//!
//! Production patterns transposed (research 2026-08-25):
//! - Loki-style monotonic replay cursors: a re-scanned batch at or below the
//!   recorded position is a no-op, never a double count;
//! - Flink-style declared watermarks: temporal freshness is an explicit
//!   per-source declaration, so a dead shipper reads as *idle*, never as
//!   absence of events — silence is not data;
//! - fail-closed coverage guard (polarité): classification may only consume
//!   signals whose coverage is sound. A blocked source must never let a key
//!   read as [`PatternClass::Fading`] — the distillation-success reading.
//!
//! Pure here: file access, offsets, and parsing stay adapter-owned in
//! `cynic-host`.

use serde::{Deserialize, Serialize};

use super::types::PatternClass;

pub const INGEST_SCHEMA: &str = "cynic.pattern_gc.ingest.v1";

/// Monotonic ingestion progress for one source (Loki replay-counter parity).
///
/// Strictly-increasing acceptance: an entry sequence at or below the cursor
/// was already accounted for (checkpoint overlap, crash replay) and must be
/// skipped, never counted twice. Within one append-only source, positions
/// are monotone, so this drops nothing new.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SourceCursor {
    /// Highest entry position already folded into observations.
    pub last_entry_seq: u64,
}

impl SourceCursor {
    /// Account for one entry. Returns `true` when the entry is new (caller
    /// counts it), `false` when it is a replay (`<= cursor`, caller skips
    /// and reports it as a duplicate).
    pub fn accept(&mut self, entry_seq: u64) -> bool {
        if entry_seq > self.last_entry_seq {
            self.last_entry_seq = entry_seq;
            true
        } else {
            false
        }
    }
}

/// Declared temporal policy for one source (watermark declaration).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatermarkPolicy {
    /// Tolerated clock skew / delivery delay before a source reads as
    /// lagging (seconds between newest event and observation time).
    pub max_out_of_orderness_s: u64,
    /// Silence beyond this span means the source is idle — an explicit
    /// unknown about current activity, never "no errors happened".
    pub idle_after_s: u64,
}

impl WatermarkPolicy {
    /// Defaults sized for interactive agent sessions (minutes of skew,
    /// a day of silence before idleness is claimed).
    pub fn observed_default() -> Self {
        Self {
            max_out_of_orderness_s: 300,
            idle_after_s: 86_400,
        }
    }
}

/// Observed temporal freshness of one source against its watermark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "freshness", rename_all = "snake_case")]
pub enum SourceFreshness {
    /// Source has produced no events at all (distinct from idle: we never
    /// observed an age).
    NoEvents,
    /// Newest event within the declared out-of-orderness tolerance.
    Current { age_s: u64 },
    /// Behind tolerance but not yet idle: data may still arrive.
    Lagging { behind_s: u64 },
    /// Silent past the idle window: current activity is UNKNOWN, and any
    /// frequency collapse over this source is unprovable.
    Idle { silent_for_s: u64 },
}

/// Pure freshness decision: age of the newest event against the policy.
/// Negative ages (future-stamped events) clamp to current — clock drift is
/// already surfaced separately as `out_of_order`.
pub fn freshness(
    max_event_ts_s: Option<i64>,
    now_s: i64,
    policy: &WatermarkPolicy,
) -> SourceFreshness {
    let Some(max_ts) = max_event_ts_s else {
        return SourceFreshness::NoEvents;
    };
    let age = (now_s - max_ts).max(0) as u64;
    if age > policy.idle_after_s {
        SourceFreshness::Idle { silent_for_s: age }
    } else if age > policy.max_out_of_orderness_s {
        SourceFreshness::Lagging {
            behind_s: age - policy.max_out_of_orderness_s,
        }
    } else {
        SourceFreshness::Current { age_s: age }
    }
}

/// Per-source honesty counters for one scan cycle. Every line the adapter
/// saw ends in exactly one of these buckets — nothing disappears silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCoverage {
    pub schema: String,
    /// Canonical source identifier (adapter-owned: path, stream, socket…).
    pub source_id: String,
    /// Total non-empty lines streamed this cycle.
    pub lines_scanned: u64,
    /// Lines without a parsable top-level timestamp.
    pub untimestamped_lines: u64,
    /// Transport-level failures (truncated write mid-scan, undecodable
    /// bytes): surfaced, never silently folded.
    pub malformed_lines: u64,
    /// Entries skipped by the replay-cursor rule (already counted).
    pub duplicates_skipped: u64,
    /// Events older than the declared out-of-orderness (counted in their
    /// window anyway — Flink allowed-lateness accounting).
    pub late_events: u64,
    /// Newest timestamp seen (unix seconds), if any.
    pub max_event_ts_s: Option<i64>,
}

impl SourceCoverage {
    pub fn new(source_id: &str) -> Self {
        Self {
            schema: INGEST_SCHEMA.to_string(),
            source_id: source_id.to_string(),
            lines_scanned: 0,
            untimestamped_lines: 0,
            malformed_lines: 0,
            duplicates_skipped: 0,
            late_events: 0,
            max_event_ts_s: None,
        }
    }
}

/// Why a source's signal is untrustworthy this cycle (fail-closed blocker).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "blocker", rename_all = "snake_case")]
pub enum CoverageBlocker {
    /// Transport corruption above the tolerated share — the parser profile
    /// may be wrong for this source.
    MalformedShareExceeded {
        malformed: u64,
        scanned: u64,
        max_share_ppm: u64,
    },
    /// Source idle beyond its watermark: collapse of its keys is not
    /// observable evidence.
    StaleSource { silent_for_s: u64 },
    /// Majority of lines carry no parsable stamp: timestamps are not where
    /// the parser profile looks — windows would be fiction.
    UntimestampedMajority { untimestamped: u64, scanned: u64 },
}

/// Verdict of the fail-closed coverage guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageVerdict {
    Sound,
    Blocked(CoverageBlocker),
}

/// Typed ingestion policy (validated inputs, unlike free-floating floats).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestPolicy {
    /// Max share of malformed lines, in parts per million.
    pub max_malformed_share_ppm: u64,
}

impl IngestPolicy {
    pub fn observed_default() -> Self {
        Self {
            max_malformed_share_ppm: 50_000, // 5%
        }
    }
}

/// The canonical fail-closed decision for one source's coverage.
///
/// Order is deliberate: staleness first (a dead source invalidates every
/// other counter's interpretation), then transport integrity, then
/// timestampability. First blocker wins; reasons stay machine-readable.
pub fn coverage_guard(
    coverage: &SourceCoverage,
    policy: &IngestPolicy,
    freshness: &SourceFreshness,
) -> CoverageVerdict {
    if let SourceFreshness::Idle { silent_for_s } = *freshness {
        return CoverageVerdict::Blocked(CoverageBlocker::StaleSource { silent_for_s });
    }
    if coverage.lines_scanned > 0 {
        if coverage.malformed_lines * 1_000_000
            > coverage.lines_scanned * policy.max_malformed_share_ppm
        {
            return CoverageVerdict::Blocked(CoverageBlocker::MalformedShareExceeded {
                malformed: coverage.malformed_lines,
                scanned: coverage.lines_scanned,
                max_share_ppm: policy.max_malformed_share_ppm,
            });
        }
        if coverage.untimestamped_lines * 2 > coverage.lines_scanned {
            return CoverageVerdict::Blocked(CoverageBlocker::UntimestampedMajority {
                untimestamped: coverage.untimestamped_lines,
                scanned: coverage.lines_scanned,
            });
        }
    }
    CoverageVerdict::Sound
}

/// Demote classes that a coverage gap makes unprovable.
///
/// `Fading` authorizes future eviction decisions, so ANY blocker turns it
/// into `Unknown` (fail-closed on the dangerous direction). Emerging stays
/// advisory under gaps — documented residual risk, bounded by emergence
/// floors upstream.
pub fn demote_on_coverage_gap(class: PatternClass, verdict: &CoverageVerdict) -> PatternClass {
    match (class, verdict) {
        (PatternClass::Fading, CoverageVerdict::Blocked(_)) => PatternClass::Unknown,
        (class, _) => class,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_accepts_strictly_increasing_positions() {
        let mut c = SourceCursor::default();
        assert!(c.accept(1));
        assert!(c.accept(2));
        assert_eq!(c.last_entry_seq, 2);
    }

    #[test]
    fn cursor_rejects_replay_at_or_below_position_never_double_counts() {
        // Loki parity: checkpoint overlap replays entries already counted.
        let mut c = SourceCursor { last_entry_seq: 10 };
        assert!(!c.accept(10));
        assert!(!c.accept(7));
        assert!(c.accept(11));
        assert_eq!(c.last_entry_seq, 11);
    }

    #[test]
    fn freshness_is_current_within_tolerance() {
        let p = WatermarkPolicy::observed_default();
        assert_eq!(
            freshness(Some(1_000_000), 1_000_100, &p),
            SourceFreshness::Current { age_s: 100 }
        );
    }

    #[test]
    fn freshness_reads_lagging_between_tolerance_and_idle() {
        let p = WatermarkPolicy::observed_default();
        // 30 min old: past 5-min tolerance, far from 1-day idle.
        assert_eq!(
            freshness(Some(1_000_000), 1_001_800, &p),
            SourceFreshness::Lagging {
                behind_s: 1_800 - 300
            }
        );
    }

    #[test]
    fn dead_shipper_reads_idle_not_absent() {
        // The safety-critical case: two days of silence is an explicit
        // unknown about the source, never "the errors stopped".
        let p = WatermarkPolicy::observed_default();
        assert_eq!(
            freshness(Some(1_000_000), 1_000_000 + 2 * 86_400, &p),
            SourceFreshness::Idle {
                silent_for_s: 2 * 86_400
            }
        );
        assert_eq!(freshness(None, 9_999_999, &p), SourceFreshness::NoEvents);
    }

    #[test]
    fn future_stamped_source_clamps_to_current() {
        let p = WatermarkPolicy::observed_default();
        assert_eq!(
            freshness(Some(1_000_500), 1_000_000, &p),
            SourceFreshness::Current { age_s: 0 }
        );
    }

    #[test]
    fn malformed_share_over_limit_blocks() {
        let mut cov = SourceCoverage::new("sessions/hermes");
        cov.lines_scanned = 100;
        cov.malformed_lines = 6; // 6% > 5% ceiling
        let v = coverage_guard(
            &cov,
            &IngestPolicy::observed_default(),
            &SourceFreshness::Current { age_s: 1 },
        );
        assert_eq!(
            v,
            CoverageVerdict::Blocked(CoverageBlocker::MalformedShareExceeded {
                malformed: 6,
                scanned: 100,
                max_share_ppm: 50_000
            })
        );
    }

    #[test]
    fn stale_source_blocks_even_when_counts_look_healthy() {
        let cov = SourceCoverage::new("kernel/journal");
        let v = coverage_guard(
            &cov,
            &IngestPolicy::observed_default(),
            &SourceFreshness::Idle {
                silent_for_s: 200_000,
            },
        );
        assert_eq!(
            v,
            CoverageVerdict::Blocked(CoverageBlocker::StaleSource {
                silent_for_s: 200_000
            })
        );
    }

    #[test]
    fn untimestamped_majority_blocks_as_parser_profile_mismatch() {
        let mut cov = SourceCoverage::new("build/raw-output");
        cov.lines_scanned = 10;
        cov.untimestamped_lines = 6;
        let v = coverage_guard(
            &cov,
            &IngestPolicy::observed_default(),
            &SourceFreshness::Current { age_s: 1 },
        );
        assert_eq!(
            v,
            CoverageVerdict::Blocked(CoverageBlocker::UntimestampedMajority {
                untimestamped: 6,
                scanned: 10
            })
        );
    }

    #[test]
    fn sound_coverage_passes_the_guard() {
        let mut cov = SourceCoverage::new("sessions/codex");
        cov.lines_scanned = 1_000;
        cov.untimestamped_lines = 20;
        cov.malformed_lines = 1;
        cov.late_events = 3;
        assert_eq!(
            coverage_guard(
                &cov,
                &IngestPolicy::observed_default(),
                &SourceFreshness::Current { age_s: 42 }
            ),
            CoverageVerdict::Sound
        );
    }

    #[test]
    fn empty_but_current_source_is_sound_zero_is_not_a_guess() {
        // A fresh source with zero lines simply has no signal yet; the guard
        // must not invent a blocker for it.
        let cov = SourceCoverage::new("brand/new");
        assert_eq!(
            coverage_guard(
                &cov,
                &IngestPolicy::observed_default(),
                &SourceFreshness::Current { age_s: 0 }
            ),
            CoverageVerdict::Sound
        );
    }

    #[test]
    fn gap_demotes_fading_to_unknown_and_leaves_other_classes() {
        let blocked = CoverageVerdict::Blocked(CoverageBlocker::StaleSource { silent_for_s: 99 });
        assert_eq!(
            demote_on_coverage_gap(PatternClass::Fading, &blocked),
            PatternClass::Unknown
        );
        assert_eq!(
            demote_on_coverage_gap(PatternClass::Recurring, &blocked),
            PatternClass::Recurring
        );
        let sound = CoverageVerdict::Sound;
        assert_eq!(
            demote_on_coverage_gap(PatternClass::Fading, &sound),
            PatternClass::Fading
        );
    }

    #[test]
    fn coverage_roundtrips_through_json_with_schema_label() {
        let mut cov = SourceCoverage::new("sessions/hermes");
        cov.lines_scanned = 5;
        let json = serde_json::to_string(&cov).unwrap();
        assert!(json.contains(INGEST_SCHEMA));
        let back: SourceCoverage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cov);
    }
}
