//! HealthGate — domain port trait for Dog health gates (circuit breakers).
//!
//! Abstraction over the circuit breaker state machine.
//! Judge uses it for evaluation filtering, health loop for probing,
//! remediation for restart decisions. Infra's CircuitBreaker implements this.

use std::time::Duration;

/// Port trait for per-Dog health state.
///
/// Implemented by `CircuitBreaker` in `infra/`. Domain and application
/// layers depend on this trait, never on the concrete infra type (Rule #32).
pub trait HealthGate: Send + Sync {
    /// Should we allow a request through this Dog?
    fn should_allow(&self) -> bool;

    /// Report a successful call — resets failure state.
    fn record_success(&self);

    /// Report a failed call — may trip the gate open.
    fn record_failure(&self);

    /// Is the gate currently open (blocking requests)?
    fn is_open(&self) -> bool;

    /// Current state as a human-readable string ("closed", "open", "half-open").
    fn state(&self) -> String;

    /// Number of consecutive failures recorded.
    fn consecutive_failures(&self) -> u32;

    /// Dog ID this gate protects.
    fn dog_id(&self) -> &str;

    /// How long the gate has been open. None if closed or half-open.
    fn opened_since(&self) -> Option<Duration>;
}
