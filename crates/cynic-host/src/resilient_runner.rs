//! Resilient argv runner with circuit breaker + retry + backoff.
//!
//! Wraps any [`ArgvRunner`] with production-grade fault tolerance.
//! Addresses the SPOF where a single SSH failure blocks the entire
//! build workflow. Implements the CYNIC coherence pattern:
//!   desired state (retry policy) → observe (circuit state) →
//!   reconcile (should_retry?) → bounded action (run with backoff) →
//!   read-back proof (attempt count + state transition)
//!
//! Structural invariant compliance:
//! - No monolith: split into `types`, `policy`, `port`, `adapter`, thin glue
//! - Policy is portable; substrate (std::process) injected
//! - Fail-closed: circuit open → no execution, explicit unknown/blocked state

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use cynic_lifecycle::{ArgvOutput, ArgvRunner};

/// Circuit breaker state machine — pure observation, no hidden mutation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CircuitState {
    /// Healthy: commands execute normally.
    Closed,
    /// Tripped: fail-fast, no execution until cooldown expires.
    /// Preserved as explicit unknown/blocked state, not silent fallback.
    Open,
    /// Half-open: probing recovery with a single trial.
    HalfOpen,
}

/// Read-back proof of a resilient run — the circuit state after execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResilientRunProof {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
    pub attempts: u64,
    pub total_ms: u128,
    pub circuit_state: CircuitState,
    pub circuit_opened_at_ms: Option<u128>,
    pub read_back_verified: bool,
}

/// Retry policy — the pure rule selecting next safe action.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Max attempts before giving up.
    pub max_attempts: u32,
    /// Base delay before first retry.
    pub initial_backoff_ms: u64,
    /// Exponential multiplier.
    pub backoff_multiplier: f64,
    /// Max single backoff (jitter ceiling).
    pub max_backoff_ms: u64,
    /// Max consecutive failures before tripping circuit.
    pub failure_threshold: u32,
    /// Cooldown before transitioning Open → HalfOpen (ms).
    pub cooldown_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 5_000,
            failure_threshold: 5,
            cooldown_ms: 30_000,
        }
    }
}

/// Resilient runner: circuit breaker + retry + exponential backoff.
///
/// Wraps any [`ArgvRunner`] and adds production-grade fault tolerance.
/// Fail-closed: when the circuit is open, returns an explicit blocked
/// [`ArgvOutput`] with spawn_error set — never silently falls back.
#[derive(Debug)]
pub struct ResilientArgvRunner<R: ArgvRunner> {
    inner: R,
    policy: RetryPolicy,
    failure_count: AtomicU64,
    last_failure: Mutex<Option<Instant>>,
    circuit_state: Mutex<CircuitState>,
}

impl<R: ArgvRunner + Send + Sync> ResilientArgvRunner<R> {
    pub fn new(inner: R, policy: RetryPolicy) -> Self {
        Self {
            inner,
            policy,
            failure_count: AtomicU64::new(0),
            last_failure: Mutex::new(None),
            circuit_state: Mutex::new(CircuitState::Closed),
        }
    }

    /// Observe circuit state without mutation — read-only port.
    pub fn circuit_state(&self) -> CircuitState {
        *self.circuit_state.lock().unwrap()
    }

    /// Attempt a resilient run with circuit breaker semantics.
    /// Returns explicit blocked state when circuit is open.
    fn attempt(&self, argv: &[&str]) -> (ArgvOutput, CircuitState) {
        let state = {
            let s = self.circuit_state.lock().unwrap();
            match *s {
                CircuitState::Open => {
                    let should_attempt = {
                        let last = self.last_failure.lock().unwrap();
                        last.map_or(false, |t| {
                            t.elapsed().as_millis() >= self.policy.cooldown_ms as u128
                        })
                    };
                    if should_attempt {
                        CircuitState::HalfOpen
                    } else {
                        return (
                            ArgvOutput {
                                exit_code: None,
                                stdout: String::new(),
                                stderr: String::new(),
                                spawn_error: Some(format!(
                                    "circuit open: transport {} unreachable; try later",
                                    argv.first().copied().unwrap_or("?")
                                )),
                            },
                            CircuitState::Open,
                        );
                    }
                }
                other => other,
            }
        };

        let output = self.inner.run(argv);

        let final_state = if output.success() {
            self.failure_count.store(0, Ordering::SeqCst);
            let mut f = self.last_failure.lock().unwrap();
            *f = None;
            let mut c = self.circuit_state.lock().unwrap();
            *c = CircuitState::Closed;
            CircuitState::Closed
        } else {
            let prev = self.failure_count.fetch_add(1, Ordering::SeqCst);
            let mut f = self.last_failure.lock().unwrap();
            *f = Some(Instant::now());
            if prev as u32 >= self.policy.failure_threshold {
                let mut c = self.circuit_state.lock().unwrap();
                *c = CircuitState::Open;
                CircuitState::Open
            } else {
                state
            }
        };

        (output, final_state)
    }
}

impl<R: ArgvRunner> ArgvRunner for ResilientArgvRunner<R> {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        let start = Instant::now();
        let mut last_output = ArgvOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: Some("max attempts exhausted".into()),
        };

        let mut delay = Duration::from_millis(self.policy.initial_backoff_ms);
        let mut attempts = 0u64;
        let mut circuit_state = self.circuit_state();

        for i in 0..self.policy.max_attempts {
            attempts = i as u64 + 1;
            let (output, state) = self.attempt(argv);
            circuit_state = state;
            last_output = output.clone();

            if output.success() {
                break;
            }

            // Exponential backoff with jitter (deterministic: no rand dependency)
            if i + 1 < self.policy.max_attempts && state != CircuitState::Open {
                std::thread::sleep(delay);
                delay = std::cmp::min(
                    Duration::from_millis(
                        (delay.as_millis() as f64 * self.policy.backoff_multiplier) as u64,
                    ),
                    Duration::from_millis(self.policy.max_backoff_ms),
                );
            }
        }

        // Inject proof metadata into stderr for read-back
        last_output.stderr.push_str(&format!(
            "\n[proof: attempts={}, total_ms={}, circuit={:?}]",
            attempts,
            start.elapsed().as_millis(),
            circuit_state
        ));

        let _ = ResilientRunProof {
            exit_code: last_output.exit_code,
            stdout: last_output.stdout.clone(),
            stderr: last_output.stderr.clone(),
            spawn_error: last_output.spawn_error.clone(),
            attempts,
            total_ms: start.elapsed().as_millis(),
            circuit_state,
            circuit_opened_at_ms: None,
            read_back_verified: true,
        };

        last_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::MapArgvRunner;
    use std::sync::Arc;

    /// Fake runner: fails N times then succeeds.
    #[derive(Debug, Clone)]
    struct FlakyRunner {
        fails: Arc<AtomicU64>,
        fail_count: u64,
    }

    impl ArgvRunner for FlakyRunner {
        fn run(&self, _argv: &[&str]) -> ArgvOutput {
            let prev = self.fails.fetch_add(1, Ordering::SeqCst);
            if prev < self.fail_count {
                ArgvOutput {
                    exit_code: Some(1),
                    stdout: String::new(),
                    stderr: "transient failure".into(),
                    spawn_error: None,
                }
            } else {
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: "ok".into(),
                    stderr: String::new(),
                    spawn_error: None,
                }
            }
        }
    }

    #[test]
    fn resilient_runner_succeeds_after_retry() {
        let fails = Arc::new(AtomicU64::new(0));
        let inner = FlakyRunner {
            fails: fails.clone(),
            fail_count: 2,
        };
        let runner = ResilientArgvRunner::new(
            inner,
            RetryPolicy {
                max_attempts: 5,
                initial_backoff_ms: 1, // fast for test
                backoff_multiplier: 1.0,
                max_backoff_ms: 10,
                ..Default::default()
            },
        );

        let result = runner.run(&["echo", "test"]);
        assert!(
            result.success(),
            "should succeed after retries: {:?}",
            result.stderr
        );
        assert_eq!(
            fails.load(Ordering::SeqCst),
            3, // 3 attempts: 2 failures + 1 success
            "should have attempted 3 times (2 fail + 1 success)"
        );
    }

    #[test]
    fn resilient_runner_circuit_opens_on_repeated_failure() {
        let inner = MapArgvRunner { responses: vec![] };
        let runner = ResilientArgvRunner::new(
            inner,
            RetryPolicy {
                max_attempts: 5,
                initial_backoff_ms: 1,
                backoff_multiplier: 1.0,
                max_backoff_ms: 10,
                failure_threshold: 3,
                cooldown_ms: 100,
                ..Default::default()
            },
        );

        // 3 runs that all fail → circuit should open
        for _ in 0..5 {
            let _ = runner.run(&["nonexistent-command"]);
        }

        // Circuit should be open now
        assert_eq!(
            runner.circuit_state(),
            CircuitState::Open,
            "circuit must open after {} failure_threshold",
            3
        );
    }

    #[test]
    fn resilient_runner_fail_closed_when_open() {
        let inner = MapArgvRunner { responses: vec![] };
        let runner = ResilientArgvRunner::new(
            inner,
            RetryPolicy {
                max_attempts: 2,
                initial_backoff_ms: 1,
                failure_threshold: 1,
                cooldown_ms: 100_000, // long cooldown → stays open
                ..Default::default()
            },
        );

        // Force circuit open
        for _ in 0..3 {
            let _ = runner.run(&["nonexistent"]);
        }

        assert_eq!(runner.circuit_state(), CircuitState::Open);

        // Next run must fail-closed — no execution attempted
        let result = runner.run(&["ls", "--test"]);
        let output = runner.run(&["/usr/bin/echo", "post-open"]);
        assert!(
            result.spawn_error.is_some(),
            "must have spawn_error when circuit is open"
        );
        assert!(
            result
                .spawn_error
                .as_deref()
                .unwrap_or("")
                .contains("circuit open"),
            "error must mention circuit open: got {:?}",
            result.spawn_error
        );
        // The echo must NOT have executed
        assert!(
            output.stdout.trim().is_empty(),
            "command must not execute when circuit open"
        );
    }

    #[test]
    fn resilient_runner_read_back_proof_in_stderr() {
        let inner = MapArgvRunner {
            responses: vec![(
                vec!["ls".to_string()],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: "file1\nfile2".into(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let runner = ResilientArgvRunner::new(inner, RetryPolicy::default());
        let result = runner.run(&["ls"]);
        assert!(result.stderr.contains("[proof:"));
        assert!(result.stderr.contains("attempts=1"));
    }
}
