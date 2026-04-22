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

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

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
/// Serializable for persistence across restarts (B5 fix — organ amnesia).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// RFC3339 timestamp of last successful score (K14: None = never succeeded).
    /// Was `Option<Instant>` — non-serializable by design. Changed to String for persistence.
    #[serde(default)]
    pub last_success: Option<String>,
    /// Cumulative latency of successful calls (ms). Used to compute mean.
    pub total_latency_ms: u64,
    /// Cumulative completion tokens from successful calls.
    /// Drives dynamic budget: completion_mean = total_completion_tokens / success_count.
    #[serde(default)]
    pub total_completion_tokens: u64,
    /// Max completion tokens observed in any single call (empirical p100).
    /// Used as conservative upper bound when calibrating per-Dog budgets.
    #[serde(default)]
    pub max_completion_tokens: u32,
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
            total_completion_tokens: 0,
            max_completion_tokens: 0,
        }
    }

    pub fn record_success(&mut self) {
        self.total_calls += 1;
        self.success_count += 1;
        self.last_success = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn record_success_with_latency(&mut self, elapsed_ms: u64) {
        self.record_success();
        self.total_latency_ms += elapsed_ms;
    }

    /// Record completion token usage from a successful evaluation.
    /// Accumulates total and tracks max for budget derivation.
    pub fn record_completion_tokens(&mut self, completion_tokens: u32) {
        self.total_completion_tokens += completion_tokens as u64;
        if completion_tokens > self.max_completion_tokens {
            self.max_completion_tokens = completion_tokens;
        }
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

    /// Mean completion tokens per successful call.
    /// Returns 0 when no successful calls recorded.
    pub fn mean_completion_tokens(&self) -> u32 {
        if self.success_count == 0 {
            return 0;
        }
        (self.total_completion_tokens / self.success_count) as u32
    }

    /// Conservative completion budget for this Dog: max observed × 1.2 safety margin.
    /// Returns None when baseline not yet established (< 20 calls) — caller must use
    /// a fallback (e.g. InferenceProfile default or backend config max_tokens).
    /// This replaces the old hardcoded `InferenceProfile::Scoring.max_tokens() = 1024`.
    pub fn completion_budget(&self) -> Option<u32> {
        if !self.is_baseline_established() {
            return None;
        }
        // 20% safety margin over observed max, capped at reasonable ceiling
        let budget = (self.max_completion_tokens as f64 * 1.2).ceil() as u32;
        Some(budget.min(4096))
    }

    /// Estimated tokens/second for this Dog from latency and completion data.
    /// Returns None when insufficient data.
    pub fn tok_per_sec(&self) -> Option<f64> {
        if self.success_count < 5 || self.total_latency_ms == 0 || self.total_completion_tokens == 0
        {
            return None;
        }
        let total_secs = self.total_latency_ms as f64 / 1000.0;
        Some(self.total_completion_tokens as f64 / total_secs)
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
/// TTL: 30 seconds. Auto-clears tripped state after TTL expires (allows recovery).
#[derive(Debug, Clone)]
pub struct ParseFailureGate {
    /// Ring buffer of recent outcomes: true = success, false = failure.
    window: VecDeque<bool>,
    capacity: usize,
    /// Timestamp when gate transitioned to tripped state.
    /// None = never tripped, or auto-cleared by TTL.
    tripped_at: Option<Instant>,
}

impl ParseFailureGate {
    /// TTL for tripped state: 30 seconds (matching CircuitBreaker cooldown).
    /// pub(crate): BackendHandle::should_allow_quality_probe needs this value.
    pub(crate) const TTL_DURATION: Duration = Duration::from_secs(30);

    pub fn new() -> Self {
        Self {
            window: VecDeque::new(),
            capacity: 10,
            tripped_at: None,
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

        // Lazy clear: drop stale tripped_at if TTL already expired. Without this,
        // a stale timestamp from a past trip could survive across the gate's lifetime
        // and prevent the next transition from setting a fresh timestamp.
        if let Some(since) = self.tripped_at
            && since.elapsed() > Self::TTL_DURATION
        {
            self.tripped_at = None;
        }

        // State transition based on current window. Centralising the write here
        // keeps is_tripped() as a pure read, and guarantees tripped_at always
        // reflects the outcome of the latest push.
        let window_bad = self.window_threshold_exceeded();
        if window_bad && self.tripped_at.is_none() {
            self.tripped_at = Some(Instant::now());
        } else if !window_bad {
            self.tripped_at = None;
        }
    }

    /// Pure window check: true when the window has ≥5 samples and >50% are failures.
    /// Does not consider TTL. Used by `push()` to drive state transitions.
    fn window_threshold_exceeded(&self) -> bool {
        if self.window.len() < 5 {
            return false;
        }
        let failures = self.window.iter().filter(|&&ok| !ok).count();
        failures as f64 / self.window.len() as f64 > 0.5
    }

    /// Gate is tripped when `tripped_at` is set and within `TTL_DURATION`.
    /// State transitions happen in `push()` — this function is a pure read.
    /// Auto-clears after `TTL_DURATION` expires (allows recovery without manual reset).
    pub fn is_tripped(&self) -> bool {
        match self.tripped_at {
            Some(since) => since.elapsed() <= Self::TTL_DURATION,
            None => false,
        }
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
    use std::time::{Duration, Instant};

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

    // ── ParseFailureGate TTL state transitions ────────────────
    //
    // These tests validate the 4 transitions of tripped_at driven by push():
    //   1. None → Some(now)  when window crosses threshold
    //   2. Some → None       when window recovers below threshold
    //   3. is_tripped returns false when Some but elapsed > TTL
    //   4. Stale Some        is lazily cleared by the next push
    //
    // Tests bypass real time by writing tripped_at directly (the test module
    // has access to private fields via `super::*`). This mirrors the pattern
    // used in infra/circuit_breaker.rs where tests mutate Inner.state directly.

    #[test]
    fn push_sets_tripped_at_when_window_crosses_threshold() {
        let mut g = ParseFailureGate::new();
        for _ in 0..4 {
            g.record_success();
        }
        for _ in 0..6 {
            g.record_failure();
        }
        // 6/10 failures = 60% > 50% → window bad → tripped_at set
        assert!(
            g.tripped_at.is_some(),
            "tripped_at must be set when window crosses threshold"
        );
        assert!(g.is_tripped());
    }

    #[test]
    fn push_clears_tripped_at_on_window_recovery() {
        let mut g = ParseFailureGate::new();
        for _ in 0..10 {
            g.record_failure();
        }
        assert!(
            g.tripped_at.is_some(),
            "precondition: gate must be tripped after 10 failures"
        );
        // 10 successes evict all failures → window = 10S → window good → cleared
        for _ in 0..10 {
            g.record_success();
        }
        assert!(
            g.tripped_at.is_none(),
            "tripped_at must be cleared on window recovery"
        );
        assert!(!g.is_tripped());
    }

    #[test]
    fn is_tripped_returns_false_after_ttl_expired() {
        let mut g = ParseFailureGate::new();
        // Simulate a trip that happened > TTL_DURATION ago via direct field access.
        // Validates the read-side TTL without needing a real sleep.
        g.tripped_at = Some(Instant::now() - Duration::from_secs(31));
        assert!(
            !g.is_tripped(),
            "is_tripped must return false once TTL has elapsed"
        );
    }

    #[test]
    fn push_lazily_clears_stale_tripped_at() {
        let mut g = ParseFailureGate::new();
        // Inject a stale tripped_at from a past trip (> TTL ago).
        g.tripped_at = Some(Instant::now() - Duration::from_secs(31));
        // Next push must clean up via the lazy-clear branch in push().
        g.record_success();
        // Window has 1 sample (<5) → window good → stale state cleared, no new trip.
        assert!(
            g.tripped_at.is_none(),
            "stale tripped_at must be cleared by subsequent push"
        );
    }
}
