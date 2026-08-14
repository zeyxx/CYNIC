// organ/health.rs — Sliding-window parse gate (organ-specific health tracking).
//
// DogStats and ScoreFailureKind live in domain/dog_health.rs (shared vocabulary).
// This file owns ParseFailureGate — organ-internal state not needed by judge.

// Re-export domain types for backward compat within organ/
pub use crate::domain::dog_health::{DogStats, ScoreFailureKind};

use std::collections::VecDeque;
use std::time::{Duration, Instant};

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

        // Lazy clear: drop stale tripped_at if TTL already expired.
        if let Some(since) = self.tripped_at
            && since.elapsed() > Self::TTL_DURATION
        {
            self.tripped_at = None;
        }

        let window_bad = self.window_threshold_exceeded();
        if window_bad && self.tripped_at.is_none() {
            self.tripped_at = Some(Instant::now());
        } else if !window_bad {
            self.tripped_at = None;
        }
    }

    /// Pure window check: true when the window has ≥5 samples and >50% are failures.
    fn window_threshold_exceeded(&self) -> bool {
        if self.window.len() < 5 {
            return false;
        }
        let failures = self.window.iter().filter(|&&ok| !ok).count();
        failures as f64 / self.window.len() as f64 > 0.5
    }

    /// Gate is tripped when `tripped_at` is set and within `TTL_DURATION`.
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
    use std::time::Duration;

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
        assert!((s.capability_limit_rate() - 0.6).abs() < 1e-10);
    }

    #[test]
    fn dog_stats_last_success_set_on_success() {
        let mut s = DogStats::new();
        assert!(s.last_success.is_none());
        s.record_success();
        assert!(s.last_success.is_some());
    }

    #[test]
    fn dog_stats_tracks_mean_latency() {
        let mut s = DogStats::new();
        s.record_success_with_latency(100);
        s.record_success_with_latency(200);
        assert!((s.mean_latency_ms() - 150.0).abs() < 1e-10);
    }

    // ── ParseFailureGate ────────────────────────────────────────

    #[test]
    fn gate_not_tripped_initially() {
        let g = ParseFailureGate::new();
        assert!(!g.is_tripped());
    }

    #[test]
    fn gate_not_tripped_under_minimum_samples() {
        let mut g = ParseFailureGate::new();
        for _ in 0..4 {
            g.record_failure();
        }
        assert!(!g.is_tripped());
    }

    #[test]
    fn gate_trips_on_majority_failures() {
        let mut g = ParseFailureGate::new();
        for _ in 0..2 {
            g.record_success();
        }
        for _ in 0..4 {
            g.record_failure();
        }
        assert!(g.is_tripped());
    }

    #[test]
    fn gate_recovers_with_successes() {
        let mut g = ParseFailureGate::new();
        for _ in 0..6 {
            g.record_failure();
        }
        assert!(g.is_tripped());
        for _ in 0..6 {
            g.record_success();
        }
        assert!(!g.is_tripped());
    }

    #[test]
    fn gate_ttl_auto_clears() {
        let mut g = ParseFailureGate::new();
        for _ in 0..6 {
            g.record_failure();
        }
        assert!(g.is_tripped());
        // Simulate TTL expiry by directly setting tripped_at in the past
        g.tripped_at = Some(std::time::Instant::now() - Duration::from_secs(31));
        assert!(!g.is_tripped());
    }
}
