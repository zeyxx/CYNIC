//! Circuit Breaker — per-Dog fault isolation.
//! Tracks failures, opens circuit after threshold, auto-recovers via probe.
//!
//! States: Closed (healthy) → Open (skip) → HalfOpen (one probe) → Closed
//! Thresholds are φ-derived: open after 3 consecutive failures,
//! cooldown = 30 seconds before half-open probe.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// φ-derived: 3 consecutive failures before opening (Fibonacci F(4))
pub const FAILURE_THRESHOLD: u32 = 3;
/// Cooldown before half-open probe attempt
pub const COOLDOWN: Duration = Duration::from_secs(30);
/// How often to re-probe healthy backends
pub const PROBE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Normal operation — requests pass through
    Closed,
    /// Too many failures — skip this Dog
    Open { since: Instant },
    /// Cooldown expired — allow one probe request
    HalfOpen,
}

/// Internal state behind a single Mutex — no race between fields.
struct Inner {
    state: CircuitState,
    consecutive_failures: u32,
}

pub struct CircuitBreaker {
    inner: Mutex<Inner>,
    dog_id: String,
}

impl CircuitBreaker {
    pub fn new(dog_id: String) -> Self {
        Self {
            inner: Mutex::new(Inner {
                state: CircuitState::Closed,
                consecutive_failures: 0,
            }),
            dog_id,
        }
    }

    /// Should we allow a request through?
    pub fn should_allow(&self) -> bool {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open { since } => {
                if since.elapsed() >= COOLDOWN {
                    inner.state = CircuitState::HalfOpen;
                    eprintln!("[CircuitBreaker] Dog '{}': Open → HalfOpen (probing)", self.dog_id);
                    true // allow one probe
                } else {
                    false // still cooling down
                }
            }
            CircuitState::HalfOpen => true, // probe in progress
        }
    }

    /// Report a successful call
    pub fn record_success(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.consecutive_failures = 0;
        if inner.state != CircuitState::Closed {
            eprintln!("[CircuitBreaker] Dog '{}': {:?} → Closed (recovered)", self.dog_id, inner.state);
            inner.state = CircuitState::Closed;
        }
    }

    /// Report a failed call
    pub fn record_failure(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.consecutive_failures += 1;
        if inner.consecutive_failures >= FAILURE_THRESHOLD && inner.state == CircuitState::Closed {
            inner.state = CircuitState::Open { since: Instant::now() };
            eprintln!("[CircuitBreaker] Dog '{}': Closed → Open after {} failures", self.dog_id, inner.consecutive_failures);
        } else if matches!(inner.state, CircuitState::HalfOpen) {
            inner.state = CircuitState::Open { since: Instant::now() };
            eprintln!("[CircuitBreaker] Dog '{}': HalfOpen → Open (probe failed)", self.dog_id);
        }
    }

    /// Current state for health reporting
    pub fn state(&self) -> String {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match inner.state {
            CircuitState::Closed => "closed".to_string(),
            CircuitState::Open { .. } => "open".to_string(),
            CircuitState::HalfOpen => "half-open".to_string(),
        }
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).consecutive_failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_closed() {
        let cb = CircuitBreaker::new("test".into());
        assert!(cb.should_allow());
        assert_eq!(cb.state(), "closed");
    }

    #[test]
    fn opens_after_threshold_failures() {
        let cb = CircuitBreaker::new("test".into());
        for _ in 0..FAILURE_THRESHOLD {
            cb.record_failure();
        }
        assert!(!cb.should_allow());
        assert_eq!(cb.state(), "open");
    }

    #[test]
    fn success_resets_failures() {
        let cb = CircuitBreaker::new("test".into());
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.consecutive_failures(), 0);
        assert!(cb.should_allow());
    }

    #[test]
    fn recovers_on_half_open_success() {
        let cb = CircuitBreaker::new("test".into());
        for _ in 0..FAILURE_THRESHOLD {
            cb.record_failure();
        }
        // Manually set to HalfOpen for testing
        cb.inner.lock().unwrap().state = CircuitState::HalfOpen;
        cb.record_success();
        assert_eq!(cb.state(), "closed");
    }

    #[test]
    fn half_open_failure_reopens() {
        let cb = CircuitBreaker::new("test".into());
        cb.inner.lock().unwrap().state = CircuitState::HalfOpen;
        cb.record_failure();
        assert_eq!(cb.state(), "open");
    }
}
