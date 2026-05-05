//! DailyBudget — per-Dog reactive call limiter.
//!
//! No hardcoded limits. The organism learns its quota from the API:
//! - Starts unlimited (limit=0) for every Dog
//! - When a 429/quota error arrives, `exhaust_from_quota()` blocks further calls
//! - Resets automatically at UTC midnight
//! - Counter tracks `used_today()` for observability

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Per-Dog daily call budget. Thread-safe, lock-free.
#[derive(Debug)]
pub struct DailyBudget {
    dog_id: String,
    /// Maximum calls allowed per UTC day. 0 = unlimited (reactive only).
    limit: u32,
    /// Calls consumed today.
    used: AtomicU32,
    /// UTC day (days since epoch) when `used` was last reset.
    reset_day: AtomicU32,
    /// Set true when the backend returns a quota error. Immediately blocks
    /// further calls even if `used < limit`. Reset on day boundary.
    quota_exhausted: AtomicBool,
}

impl DailyBudget {
    /// Create a new budget. `limit = 0` means unlimited (reactive only).
    pub fn new(dog_id: &str, limit: u32) -> Self {
        Self {
            dog_id: dog_id.to_string(),
            limit,
            used: AtomicU32::new(0),
            reset_day: AtomicU32::new(Self::current_utc_day()),
            quota_exhausted: AtomicBool::new(false),
        }
    }

    /// Try to consume one call. Returns true if allowed, false if exhausted.
    /// Automatically resets on day boundary.
    pub fn try_consume(&self) -> bool {
        self.maybe_reset();
        // Quota exhaustion from API takes precedence
        if self.quota_exhausted.load(Ordering::Relaxed) {
            return false;
        }
        // Always increment counter (for observability)
        self.used.fetch_add(1, Ordering::Relaxed);
        // If a hard limit is set, enforce it
        if self.limit > 0 && self.used.load(Ordering::Relaxed) > self.limit {
            self.used.fetch_sub(1, Ordering::Relaxed);
            return false;
        }
        true
    }

    /// Check if budget is available WITHOUT consuming.
    pub fn has_budget(&self) -> bool {
        self.maybe_reset();
        if self.quota_exhausted.load(Ordering::Relaxed) {
            return false;
        }
        if self.limit == 0 {
            return true;
        }
        self.used.load(Ordering::Relaxed) < self.limit
    }

    /// Remaining calls today (u32::MAX if unlimited and not quota-exhausted).
    pub fn remaining(&self) -> u32 {
        self.maybe_reset();
        if self.quota_exhausted.load(Ordering::Relaxed) {
            return 0;
        }
        if self.limit == 0 {
            return u32::MAX;
        }
        self.limit.saturating_sub(self.used.load(Ordering::Relaxed))
    }

    /// Total limit per day (0 = unlimited/reactive).
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

    /// Called when the backend returns a quota/rate-limit error (HTTP 429).
    /// Immediately blocks further calls until the next UTC day reset.
    /// The organism learns its real quota from the API, not from config.
    pub fn exhaust_from_quota(&self) {
        self.quota_exhausted.store(true, Ordering::Relaxed);
        tracing::info!(
            dog_id = %self.dog_id,
            used = self.used.load(Ordering::Relaxed),
            "budget exhausted by API quota — no further calls until reset"
        );
    }

    /// True if exhaustion was caused by an API quota error (not just counter).
    pub fn is_quota_exhausted(&self) -> bool {
        self.quota_exhausted.load(Ordering::Relaxed)
    }

    /// Reset counter and quota flag if we've crossed into a new UTC day.
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
            self.quota_exhausted.store(false, Ordering::Relaxed);
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
        for _ in 0..100 {
            assert!(b.try_consume());
        }
        assert_eq!(b.used_today(), 100);
    }

    #[test]
    fn quota_exhaustion_blocks() {
        let b = DailyBudget::new("test", 0);
        assert!(b.try_consume());
        assert!(b.try_consume());
        b.exhaust_from_quota();
        assert!(!b.try_consume());
        assert!(!b.has_budget());
        assert_eq!(b.remaining(), 0);
        assert!(b.is_quota_exhausted());
        // Counter shows calls before exhaustion
        assert_eq!(b.used_today(), 2);
    }

    #[test]
    fn hard_limit_enforced() {
        let b = DailyBudget::new("test", 3);
        assert!(b.try_consume());
        assert!(b.try_consume());
        assert!(b.try_consume());
        assert!(!b.try_consume());
        assert_eq!(b.remaining(), 0);
    }

    #[test]
    fn has_budget_reflects_state() {
        let b = DailyBudget::new("test", 0);
        assert!(b.has_budget());
        b.exhaust_from_quota();
        assert!(!b.has_budget());
    }
}
