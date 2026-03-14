//! Circuit Breaker — per-Dog fault isolation.
//! Tracks failures, opens circuit after threshold, auto-recovers via probe.
//!
//! States: Closed (healthy) → Open (skip) → HalfOpen (one probe) → Closed
//! Thresholds are φ-derived: open after 3 consecutive failures,
//! cooldown = 30 seconds before half-open probe.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// φ-derived: 3 consecutive failures before opening (Fibonacci F(4))
const FAILURE_THRESHOLD: u32 = 3;
/// Cooldown before half-open probe attempt
const COOLDOWN: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Normal operation — requests pass through
    Closed,
    /// Too many failures — skip this Dog
    Open { since: Instant },
    /// Cooldown expired — allow one probe request
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Mutex<CircuitState>,
    consecutive_failures: Mutex<u32>,
    dog_id: String,
}

impl CircuitBreaker {
    pub fn new(dog_id: String) -> Self {
        Self {
            state: Mutex::new(CircuitState::Closed),
            consecutive_failures: Mutex::new(0),
            dog_id,
        }
    }

    /// Should we allow a request through?
    pub fn should_allow(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open { since } => {
                if since.elapsed() >= COOLDOWN {
                    *state = CircuitState::HalfOpen;
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
        let mut state = self.state.lock().unwrap();
        let mut failures = self.consecutive_failures.lock().unwrap();
        *failures = 0;
        if *state != CircuitState::Closed {
            eprintln!("[CircuitBreaker] Dog '{}': {:?} → Closed (recovered)", self.dog_id, *state);
            *state = CircuitState::Closed;
        }
    }

    /// Report a failed call
    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        let mut failures = self.consecutive_failures.lock().unwrap();
        *failures += 1;
        if *failures >= FAILURE_THRESHOLD && *state == CircuitState::Closed {
            *state = CircuitState::Open { since: Instant::now() };
            eprintln!("[CircuitBreaker] Dog '{}': Closed → Open after {} failures", self.dog_id, *failures);
        } else if matches!(*state, CircuitState::HalfOpen) {
            *state = CircuitState::Open { since: Instant::now() };
            eprintln!("[CircuitBreaker] Dog '{}': HalfOpen → Open (probe failed)", self.dog_id);
        }
    }

    /// Current state for health reporting
    pub fn state(&self) -> String {
        let state = self.state.lock().unwrap();
        match *state {
            CircuitState::Closed => "closed".to_string(),
            CircuitState::Open { .. } => "open".to_string(),
            CircuitState::HalfOpen => "half-open".to_string(),
        }
    }

    pub fn consecutive_failures(&self) -> u32 {
        *self.consecutive_failures.lock().unwrap()
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
        // Force open with immediate cooldown
        for _ in 0..FAILURE_THRESHOLD {
            cb.record_failure();
        }
        // Manually set to HalfOpen for testing
        *cb.state.lock().unwrap() = CircuitState::HalfOpen;
        cb.record_success();
        assert_eq!(cb.state(), "closed");
    }

    #[test]
    fn half_open_failure_reopens() {
        let cb = CircuitBreaker::new("test".into());
        *cb.state.lock().unwrap() = CircuitState::HalfOpen;
        cb.record_failure();
        assert_eq!(cb.state(), "open");
    }
}
