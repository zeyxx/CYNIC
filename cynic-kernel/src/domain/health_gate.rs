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

/// Determine system health status from Dogs, storage, probes, and background tasks.
///
/// Gate: counts only circuit=closed dogs, not total registered dogs.
/// - critical (503): zero healthy dogs OR storage down
/// - degraded (503): only 1 healthy dog OR majority down OR probes degraded OR tasks stale
/// - sovereign (200): majority healthy + storage up + probes ok + no critical tasks
pub fn system_health_status(
    healthy_dogs: usize,
    total_dogs: usize,
    storage_ok: bool,
    probes_degraded: bool,
    tasks_stale: bool,
) -> (&'static str, bool) {
    if healthy_dogs == 0 || !storage_ok {
        ("critical", false)
    } else if healthy_dogs == 1
        || (total_dogs > 0 && healthy_dogs * 2 < total_dogs)
        || probes_degraded
        || tasks_stale
    {
        ("degraded", false)
    } else {
        ("sovereign", true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_zero_dogs_is_critical() {
        assert_eq!(
            system_health_status(0, 0, true, false, false),
            ("critical", false)
        );
    }

    #[test]
    fn health_storage_down_is_critical() {
        assert_eq!(
            system_health_status(5, 5, false, false, false),
            ("critical", false)
        );
    }

    #[test]
    fn health_one_healthy_dog_is_degraded() {
        assert_eq!(
            system_health_status(1, 5, true, false, false),
            ("degraded", false)
        );
    }

    #[test]
    fn health_5_dogs_3_critical_is_degraded() {
        assert_eq!(
            system_health_status(2, 5, true, false, false),
            ("degraded", false)
        );
    }

    #[test]
    fn health_5_dogs_1_critical_is_sovereign() {
        assert_eq!(
            system_health_status(4, 5, true, false, false),
            ("sovereign", true)
        );
    }

    #[test]
    fn health_all_healthy_is_sovereign() {
        assert_eq!(
            system_health_status(5, 5, true, false, false),
            ("sovereign", true)
        );
    }

    #[test]
    fn health_2_of_2_is_sovereign() {
        assert_eq!(
            system_health_status(2, 2, true, false, false),
            ("sovereign", true)
        );
    }

    #[test]
    fn health_probes_degraded_is_degraded() {
        assert_eq!(
            system_health_status(5, 5, true, true, false),
            ("degraded", false)
        );
    }

    #[test]
    fn health_tasks_stale_is_degraded() {
        assert_eq!(
            system_health_status(5, 5, true, false, true),
            ("degraded", false)
        );
    }
}
