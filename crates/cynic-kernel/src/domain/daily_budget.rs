//! DailyBudget — per-Dog daily call limiter.
//!
//! Tracks calls per UTC day. When budget is exhausted, the Dog is skipped.
//! Resets automatically at UTC midnight. Exposed via /health for observability.
//!
//! Designed for quota-constrained backends (e.g. Gemini free tier: ~1500/day).
//! Dogs without a budget (None) are unlimited.

use std::sync::atomic::{AtomicU32, Ordering};

/// Per-Dog daily call budget. Thread-safe, lock-free.
#[derive(Debug)]
pub struct DailyBudget {
    dog_id: String,
    /// Maximum calls allowed per UTC day. 0 = unlimited (no budget).
    limit: u32,
    /// Calls consumed today.
    used: AtomicU32,
    /// UTC day (days since epoch) when `used` was last reset.
    reset_day: AtomicU32,
}

impl DailyBudget {
    /// Create a new budget. `limit = 0` means unlimited.
    pub fn new(dog_id: &str, limit: u32) -> Self {
        Self {
            dog_id: dog_id.to_string(),
            limit,
            used: AtomicU32::new(0),
            reset_day: AtomicU32::new(Self::current_utc_day()),
        }
    }

    /// Try to consume one call. Returns true if allowed, false if budget exhausted.
    /// Automatically resets on day boundary.
    pub fn try_consume(&self) -> bool {
        if self.limit == 0 {
            return true; // unlimited
        }
        self.maybe_reset();
        let prev = self.used.fetch_add(1, Ordering::Relaxed);
        if prev >= self.limit {
            // Over budget — undo the increment
            self.used.fetch_sub(1, Ordering::Relaxed);
            false
        } else {
            true
        }
    }

    /// Check if budget is available WITHOUT consuming.
    pub fn has_budget(&self) -> bool {
        if self.limit == 0 {
            return true;
        }
        self.maybe_reset();
        self.used.load(Ordering::Relaxed) < self.limit
    }

    /// Remaining calls today.
    pub fn remaining(&self) -> u32 {
        if self.limit == 0 {
            return u32::MAX; // unlimited
        }
        self.maybe_reset();
        self.limit.saturating_sub(self.used.load(Ordering::Relaxed))
    }

    /// Total limit per day (0 = unlimited).
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Calls used today.
    pub fn used_today(&self) -> u32 {
        self.maybe_reset();
        self.used.load(Ordering::Relaxed)
    }

    pub fn dog_id(&self) -> &str {
        &self.dog_id
    }

    /// Reset counter if we've crossed into a new UTC day.
    fn maybe_reset(&self) {
        let today = Self::current_utc_day();
        let last_reset = self.reset_day.load(Ordering::Relaxed);
        if today != last_reset
            && self
                .reset_day
                .compare_exchange(last_reset, today, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            self.used.store(0, Ordering::Relaxed);
        }
    }

    /// Current UTC day as days since Unix epoch.
    fn current_utc_day() -> u32 {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        (secs / 86400) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_always_allows() {
        let b = DailyBudget::new("test", 0);
        for _ in 0..10000 {
            assert!(b.try_consume());
        }
    }

    #[test]
    fn budget_exhausts_at_limit() {
        let b = DailyBudget::new("test", 5);
        for _ in 0..5 {
            assert!(b.try_consume());
        }
        assert!(!b.try_consume());
        assert_eq!(b.remaining(), 0);
        assert_eq!(b.used_today(), 5);
    }

    #[test]
    fn remaining_reports_correctly() {
        let b = DailyBudget::new("test", 10);
        assert_eq!(b.remaining(), 10);
        b.try_consume();
        b.try_consume();
        assert_eq!(b.remaining(), 8);
    }

    #[test]
    fn has_budget_reflects_state() {
        let b = DailyBudget::new("test", 2);
        assert!(b.has_budget());
        b.try_consume();
        assert!(b.has_budget());
        b.try_consume();
        assert!(!b.has_budget());
    }
}
