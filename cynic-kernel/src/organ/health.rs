// organ/health.rs — Per-Dog quality counters and sliding-window parse gate.
//
// Diagnostic findings (2026-04-01) inform this design:
// - Small models (<10B, no CoT) COLLAPSE on bad content: all axioms → same value (var=0).
//   This is a model capability limit, not a config problem. Track separately from zero floods.
// - Saturation on easy content: all LLM dogs saturate at phi_bound ceiling (0.618).
//   validate_scores() passes but discrimination is lost. Tracked via success_count only.
// - Welford drift detection is Phase 2. Phase 1 needs counters + auto-degrade.
//
// K14: unknown rates default to 0.0 (pessimistic), unknown latency to u32::MAX.

use std::collections::VecDeque;
use std::time::Instant;

/// Failure type classification — different causes, different remediation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreFailureKind {
    /// >3 axioms at exactly 0.0. Model assigns zero to everything "bad". Capability limit.
    ZeroFlood,
    /// All axiom scores identical (variance ≈ 0). Model collapses to single value. Capability limit.
    Collapse,
    /// JSON not found, malformed, or no numeric scores after lenient parse.
    ParseError,
    /// Backend unreachable or timed out.
    Timeout,
    /// Backend returned HTTP error or was unreachable. Infrastructure issue, not model quality.
    ApiError,
}

/// Per-Dog rolling counters. Updated after each Dog evaluation.
/// Does NOT lock — caller (Judge) holds the Arc<Mutex<DogStats>>.
#[derive(Debug, Clone)]
pub struct DogStats {
    /// Total calls attempted (success + all failure kinds).
    pub total_calls: u64,
    /// Calls that produced valid, differentiated scores (passed validate_scores).
    pub success_count: u64,
    /// Zero-flood failures: >3 axioms at 0.0.
    pub zero_flood_count: u64,
    /// Collapse failures: all axioms same value (variance < threshold).
    pub collapse_count: u64,
    /// Parse/JSON failures.
    pub parse_error_count: u64,
    /// Timeout failures.
    pub timeout_count: u64,
    /// API/infrastructure failures (backend unreachable, HTTP errors).
    pub api_error_count: u64,
    /// Timestamp of last successful score (K14: None = never succeeded).
    pub last_success: Option<Instant>,
    /// Cumulative latency of successful calls (ms). Used to compute mean.
    pub total_latency_ms: u64,
}

impl DogStats {
    pub fn new() -> Self {
        Self {
            total_calls: 0,
            success_count: 0,
            zero_flood_count: 0,
            collapse_count: 0,
            parse_error_count: 0,
            timeout_count: 0,
            api_error_count: 0,
            last_success: None,
            total_latency_ms: 0,
        }
    }

    pub fn record_success(&mut self) {
        self.total_calls += 1;
        self.success_count += 1;
        self.last_success = Some(Instant::now());
    }

    pub fn record_success_with_latency(&mut self, elapsed_ms: u64) {
        self.record_success();
        self.total_latency_ms += elapsed_ms;
    }

    /// Mean latency of successful calls in milliseconds.
    /// Returns 0.0 when no successful calls recorded.
    pub fn mean_latency_ms(&self) -> f64 {
        if self.success_count == 0 {
            return 0.0;
        }
        self.total_latency_ms as f64 / self.success_count as f64
    }

    pub fn record_failure(&mut self, kind: ScoreFailureKind) {
        self.total_calls += 1;
        match kind {
            ScoreFailureKind::ZeroFlood => self.zero_flood_count += 1,
            ScoreFailureKind::Collapse => self.collapse_count += 1,
            ScoreFailureKind::ParseError => self.parse_error_count += 1,
            ScoreFailureKind::Timeout => self.timeout_count += 1,
            ScoreFailureKind::ApiError => self.api_error_count += 1,
        }
    }

    /// Fraction of calls that produced valid scores. K14: 0.0 when unknown (no calls yet).
    pub fn json_valid_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0; // K14: unknown = pessimistic
        }
        self.success_count as f64 / self.total_calls as f64
    }

    /// Fraction of calls that are capability-limit failures (zero flood + collapse).
    /// High value → model can't discriminate on this content type. Not fixable with config.
    pub fn capability_limit_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        (self.zero_flood_count + self.collapse_count) as f64 / self.total_calls as f64
    }

    /// Has seen enough calls to be statistically meaningful.
    pub fn is_baseline_established(&self) -> bool {
        self.total_calls >= 20
    }
}

impl Default for DogStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Sliding-window gate — trips when parse failure rate exceeds threshold over last N calls.
/// Degrade-on-trip: the organ should mark backend as Degraded when gate fires.
///
/// Window size: 10 calls. Trip threshold: >50% failures.
/// Minimum samples: 5 (avoid tripping on first bad call).
#[derive(Debug, Clone)]
pub struct ParseFailureGate {
    /// Ring buffer of recent outcomes: true = success, false = failure.
    window: VecDeque<bool>,
    capacity: usize,
}

impl ParseFailureGate {
    pub fn new() -> Self {
        Self {
            window: VecDeque::new(),
            capacity: 10,
        }
    }

    pub fn record_success(&mut self) {
        self.push(true);
    }

    pub fn record_failure(&mut self) {
        self.push(false);
    }

    fn push(&mut self, ok: bool) {
        if self.window.len() >= self.capacity {
            self.window.pop_front();
        }
        self.window.push_back(ok);
    }

    /// Gate is tripped when >50% of the last N calls failed (min 5 samples required).
    pub fn is_tripped(&self) -> bool {
        if self.window.len() < 5 {
            return false;
        }
        let failures = self.window.iter().filter(|&&ok| !ok).count();
        failures as f64 / self.window.len() as f64 > 0.5
    }

    /// Current failure rate in the window. 0.0 when no data.
    pub fn failure_rate(&self) -> f64 {
        if self.window.is_empty() {
            return 0.0;
        }
        let failures = self.window.iter().filter(|&&ok| !ok).count();
        failures as f64 / self.window.len() as f64
    }

    pub fn sample_count(&self) -> usize {
        self.window.len()
    }
}

impl Default for ParseFailureGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DogStats ────────────────────────────────────────────────

    #[test]
    fn dog_stats_new_is_pessimistic() {
        let s = DogStats::new();
        assert_eq!(s.json_valid_rate(), 0.0); // K14: unknown = 0
        assert!(!s.is_baseline_established());
    }

    #[test]
    fn dog_stats_all_success_rate_is_one() {
        let mut s = DogStats::new();
        for _ in 0..20 {
            s.record_success();
        }
        assert!((s.json_valid_rate() - 1.0).abs() < 1e-10);
        assert!(s.is_baseline_established());
    }

    #[test]
    fn dog_stats_mixed_rate() {
        let mut s = DogStats::new();
        for _ in 0..8 {
            s.record_success();
        }
        for _ in 0..2 {
            s.record_failure(ScoreFailureKind::ZeroFlood);
        }
        assert!((s.json_valid_rate() - 0.8).abs() < 1e-10);
    }

    #[test]
    fn dog_stats_capability_limit_rate_zero_floods() {
        let mut s = DogStats::new();
        for _ in 0..4 {
            s.record_success();
        }
        for _ in 0..4 {
            s.record_failure(ScoreFailureKind::ZeroFlood);
        }
        for _ in 0..2 {
            s.record_failure(ScoreFailureKind::Collapse);
        }
        // 6/10 are capability limits
        assert!((s.capability_limit_rate() - 0.6).abs() < 1e-10);
    }

    #[test]
    fn dog_stats_last_success_set_on_success() {
        let mut s = DogStats::new();
        assert!(s.last_success.is_none()); // K14: never succeeded
        s.record_success();
        assert!(s.last_success.is_some());
    }

    #[test]
    fn dog_stats_tracks_mean_latency() {
        let mut s = DogStats::new();
        assert_eq!(s.mean_latency_ms(), 0.0);
        s.record_success_with_latency(100);
        s.record_success_with_latency(200);
        s.record_success_with_latency(300);
        assert!((s.mean_latency_ms() - 200.0).abs() < 1e-10);
    }

    #[test]
    fn dog_stats_baseline_after_20_calls() {
        let mut s = DogStats::new();
        for _ in 0..19 {
            s.record_failure(ScoreFailureKind::ParseError);
        }
        assert!(!s.is_baseline_established());
        s.record_failure(ScoreFailureKind::ParseError);
        assert!(s.is_baseline_established());
    }

    // ── ParseFailureGate ────────────────────────────────────────

    #[test]
    fn gate_new_is_not_tripped() {
        let g = ParseFailureGate::new();
        assert!(!g.is_tripped());
        assert_eq!(g.failure_rate(), 0.0);
    }

    #[test]
    fn gate_needs_minimum_5_samples() {
        let mut g = ParseFailureGate::new();
        for _ in 0..4 {
            g.record_failure();
        }
        assert!(!g.is_tripped()); // < 5 samples → never trips
    }

    #[test]
    fn gate_trips_above_50_percent() {
        let mut g = ParseFailureGate::new();
        for _ in 0..5 {
            g.record_success();
        }
        assert!(!g.is_tripped());
        for _ in 0..6 {
            g.record_failure();
        }
        // 6/11 = 54.5% failures > 50% → tripped
        assert!(g.is_tripped());
    }

    #[test]
    fn gate_does_not_trip_at_exactly_50_percent() {
        let mut g = ParseFailureGate::new();
        for _ in 0..5 {
            g.record_success();
        }
        for _ in 0..5 {
            g.record_failure();
        }
        // 5/10 = 50%, threshold is STRICTLY > 50%
        assert!(!g.is_tripped());
    }

    #[test]
    fn gate_sliding_window_evicts_old_entries() {
        let mut g = ParseFailureGate::new();
        // Add 10 failures (filling window)
        for _ in 0..10 {
            g.record_failure();
        }
        assert!(g.is_tripped());
        // Add 10 successes (overwrite the failures)
        for _ in 0..10 {
            g.record_success();
        }
        // Window now has 10 successes → not tripped
        assert!(!g.is_tripped());
        assert_eq!(g.sample_count(), 10);
    }

    #[test]
    fn gate_failure_rate_correct() {
        let mut g = ParseFailureGate::new();
        for _ in 0..7 {
            g.record_success();
        }
        for _ in 0..3 {
            g.record_failure();
        }
        assert!((g.failure_rate() - 0.3).abs() < 1e-10);
    }
}
