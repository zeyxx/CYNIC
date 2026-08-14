//! EpocheThreshold — data-driven suspension window for Dog disagreement.
//!
//! When Dogs disagree beyond a data-derived P95 threshold, verdict is suspended
//! (EPOCHÉ) rather than forcing a conclusion from conflicting signals.
//!
//! Source: Sextus Empiricus PH I.4 — isosthéneia (equipollence) → epoché (suspension).
//! The threshold is not hardcoded — it emerges from observed Dog behavior over a
//! sliding window, so calibration is automatic and empirically grounded.

use std::collections::VecDeque;

// ── CONSTANTS ──────────────────────────────────────────────

/// Minimum samples required before a threshold can be computed.
/// Below this count, `threshold()` returns `None` (not enough evidence).
const MIN_SAMPLES: usize = 10;

/// Percentile used for the suspension threshold.
const PERCENTILE: f64 = 0.95;

// ── STRUCT ─────────────────────────────────────────────────

/// Sliding-window P95 tracker for Dog max_disagreement values.
///
/// Records observed disagreement values and derives a data-driven suspension
/// threshold (P95 of the window). When a new judgment's max_disagreement
/// exceeds this threshold AND at least 2 Dogs voted, the verdict is suspended.
///
/// The window is bounded to capacity N: oldest values are evicted when full.
#[derive(Debug)]
pub struct EpocheThreshold {
    /// Ring buffer of recent max_disagreement values (capacity = N).
    window: VecDeque<f64>,
    /// Maximum number of samples in the window.
    capacity: usize,
}

impl EpocheThreshold {
    /// Create a new threshold tracker with the given window capacity.
    ///
    /// `capacity` controls how many recent samples are retained. Older values
    /// are evicted once the window is full (ring-buffer semantics).
    pub fn new(capacity: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Record a new max_disagreement observation.
    ///
    /// Pushes the value into the window and evicts the oldest entry if the
    /// window is already at capacity.
    pub fn record(&mut self, max_disagreement: f64) {
        if self.window.len() >= self.capacity {
            self.window.pop_front();
        }
        self.window.push_back(max_disagreement);
    }

    /// Compute the P95 of the current window.
    ///
    /// Returns `None` if the window contains fewer than `MIN_SAMPLES` entries —
    /// the distribution isn't meaningful until enough data has been observed.
    ///
    /// Off-by-one: `idx = floor((n - 1) * 0.95)` so that P95 of a 100-sample
    /// uniform distribution correctly returns the 95th value (not the 96th).
    pub fn threshold(&self) -> Option<f64> {
        if self.window.len() < MIN_SAMPLES {
            return None;
        }

        let mut sorted: Vec<f64> = self.window.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx = (((sorted.len() - 1) as f64) * PERCENTILE).floor() as usize;
        Some(sorted[idx])
    }

    /// Whether the given disagreement level warrants suspension.
    ///
    /// Returns `true` iff:
    /// - At least 2 Dogs voted (`voter_count >= 2`), AND
    /// - A threshold can be computed (window has ≥ MIN_SAMPLES), AND
    /// - `max_disagreement` exceeds the P95 threshold.
    ///
    /// A single Dog can never disagree with itself, so `voter_count < 2` never suspends.
    pub fn should_suspend(&self, max_disagreement: f64, voter_count: usize) -> bool {
        if voter_count < 2 {
            return false;
        }
        match self.threshold() {
            Some(t) => max_disagreement > t,
            None => false,
        }
    }

    /// Number of samples currently in the window.
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Whether the window is empty.
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Clone the current window contents for propagation (e.g. without-dog scenarios).
    ///
    /// # WHY: used in Task 3 (Judge::without_dog propagation) — pub(crate) until wired.
    #[allow(dead_code)] // WHY: used in Task 3 (Judge::without_dog propagation)
    pub(crate) fn snapshot(&self) -> Vec<f64> {
        self.window.iter().copied().collect()
    }
}

// ── TESTS ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // T1 — empty window has no threshold.
    #[test]
    fn empty_window_returns_none() {
        let t = EpocheThreshold::new(100);
        assert!(t.threshold().is_none());
    }

    // T2 — window with fewer than MIN_SAMPLES has no threshold.
    #[test]
    fn below_min_samples_returns_none() {
        let mut t = EpocheThreshold::new(100);
        for i in 0..9 {
            t.record(i as f64 * 0.1);
        }
        assert_eq!(t.len(), 9);
        assert!(t.threshold().is_none());
    }

    // T3 — 100 values uniformly spaced 0.01..=1.00. P95 index = floor(99 * 0.95) = 94.
    // The 95th value (0-indexed at 94) in a sorted 0.01..=1.00 range is 0.95.
    #[test]
    fn p95_computed_correctly() {
        let mut t = EpocheThreshold::new(200);
        for i in 1..=100 {
            t.record(i as f64 * 0.01);
        }
        let thresh = t.threshold().expect("100 samples → threshold must exist");
        // idx = floor(99 * 0.95) = floor(94.05) = 94 → sorted[94] = 0.95
        assert!(
            (thresh - 0.95).abs() < 1e-10,
            "P95 of 0.01..1.00 must be 0.95, got {thresh}"
        );
    }

    // T4 — window evicts oldest when full.
    //
    // Fill 100 slots with 0.1, then push 6 × 0.9 (evicts 6 oldest 0.1 entries).
    // Window = [0.1 × 94, 0.9 × 6]. sorted: indices 0-93 = 0.1, indices 94-99 = 0.9.
    // idx = floor(99 * 0.95) = floor(94.05) = 94 → sorted[94] = 0.9.
    #[test]
    fn window_evicts_oldest() {
        let mut t = EpocheThreshold::new(100);
        for _ in 0..100 {
            t.record(0.1);
        }
        // Push 6 × 0.9 — evicts 6 oldest 0.1 entries
        for _ in 0..6 {
            t.record(0.9);
        }
        assert_eq!(t.len(), 100, "capacity must be maintained");

        let thresh = t.threshold().expect("100 samples → threshold must exist");
        // sorted: [0.1 × 94, 0.9 × 6]. idx = 94 → sorted[94] = 0.9
        assert!(
            (thresh - 0.9).abs() < 1e-10,
            "P95 after eviction must be 0.9, got {thresh}"
        );
    }

    // T5 — should_suspend: single voter never suspends, multi-voter above threshold suspends.
    #[test]
    fn should_suspend_respects_threshold_and_voter_count() {
        let mut t = EpocheThreshold::new(100);
        // Fill with low disagreement values so threshold ≈ 0.095
        for i in 1..=100 {
            t.record(i as f64 * 0.001);
        }
        // P95 = sorted[94] = 0.095
        let thresh = t.threshold().expect("threshold must exist");

        // Single voter — never suspends regardless of disagreement.
        assert!(
            !t.should_suspend(thresh + 1.0, 1),
            "single voter must never suspend"
        );

        // Multi-voter below threshold — no suspension.
        assert!(
            !t.should_suspend(thresh - 0.001, 2),
            "below threshold must not suspend"
        );

        // Multi-voter above threshold — suspend.
        assert!(
            t.should_suspend(thresh + 0.001, 2),
            "above threshold with 2 voters must suspend"
        );

        // Multi-voter at exactly the threshold — not above, so no suspension.
        assert!(
            !t.should_suspend(thresh, 3),
            "exactly at threshold must not suspend (strict >)"
        );
    }

    // T6 — snapshot returns the window contents in insertion order.
    #[test]
    fn snapshot_returns_window_contents() {
        let mut t = EpocheThreshold::new(50);
        let values = vec![0.1, 0.3, 0.5, 0.7, 0.9];
        for &v in &values {
            t.record(v);
        }
        let snap = t.snapshot();
        assert_eq!(snap, values, "snapshot must match recorded values in order");
    }
}
