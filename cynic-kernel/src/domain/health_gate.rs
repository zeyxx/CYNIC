//! HealthGate — domain port trait for Dog health gates (circuit breakers).
//!
//! Abstraction over the circuit breaker state machine.
//! Judge uses it for evaluation filtering, health loop for probing,
//! remediation for restart decisions. Infra's CircuitBreaker implements this.

use std::time::Duration;

use super::contract::ContractDelta;

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

/// Count healthy (circuit=closed) Dogs from a health snapshot.
/// Returns `(healthy_count, total_count)`.
pub fn count_healthy_dogs(dog_health: &[(String, String, u32)]) -> (usize, usize) {
    let healthy = dog_health
        .iter()
        .filter(|(_, circuit, _)| circuit == "closed")
        .count();
    (healthy, dog_health.len())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HealthCause {
    NoHealthyDogs,
    StorageUnavailable,
    SingleHealthyDog {
        healthy_dogs: usize,
        total_dogs: usize,
    },
    MajorityDogsDown {
        healthy_dogs: usize,
        total_dogs: usize,
    },
    ProbesDegraded,
    ReadinessTasksStale {
        tasks: Vec<&'static str>,
    },
    /// Contract declares Dogs that are missing from the live roster.
    ContractGap {
        missing: Vec<String>,
        expected: usize,
        actual: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HealthAssessment {
    pub status: &'static str,
    pub healthy: bool,
    pub causes: Vec<HealthCause>,
}

/// Determine system health status from Dogs, storage, probes, background tasks,
/// and optionally a SystemContract (self-model).
///
/// Gate: counts only circuit=closed dogs, not total registered dogs.
/// - critical (503): zero healthy dogs OR storage down
/// - degraded (503): only 1 healthy dog OR majority down OR probes degraded OR tasks stale
/// - operational (200): above threshold, but contract has gaps (missing expected Dogs)
/// - sovereign (200): contract fulfilled (or no contract) + all other checks pass
pub fn system_health_assessment(
    healthy_dogs: usize,
    total_dogs: usize,
    storage_ok: bool,
    probes_degraded: bool,
    stale_tasks: &[&'static str],
) -> HealthAssessment {
    system_health_assessment_with_contract(
        healthy_dogs,
        total_dogs,
        storage_ok,
        probes_degraded,
        stale_tasks,
        None,
    )
}

/// Full assessment with optional contract comparison.
pub fn system_health_assessment_with_contract(
    healthy_dogs: usize,
    total_dogs: usize,
    storage_ok: bool,
    probes_degraded: bool,
    stale_tasks: &[&'static str],
    contract_delta: Option<&ContractDelta>,
) -> HealthAssessment {
    let mut causes = Vec::new();

    if healthy_dogs == 0 {
        causes.push(HealthCause::NoHealthyDogs);
    }
    if !storage_ok {
        causes.push(HealthCause::StorageUnavailable);
    }
    if healthy_dogs == 1 && total_dogs > 0 {
        causes.push(HealthCause::SingleHealthyDog {
            healthy_dogs,
            total_dogs,
        });
    }
    if total_dogs > 0 && healthy_dogs > 0 && healthy_dogs * 2 < total_dogs {
        causes.push(HealthCause::MajorityDogsDown {
            healthy_dogs,
            total_dogs,
        });
    }
    if probes_degraded {
        causes.push(HealthCause::ProbesDegraded);
    }
    if !stale_tasks.is_empty() {
        causes.push(HealthCause::ReadinessTasksStale {
            tasks: stale_tasks.to_vec(),
        });
    }

    let contract_gap = contract_delta.is_some_and(|d| !d.fulfilled);
    if let Some(delta) = contract_delta
        && !delta.fulfilled
    {
        causes.push(HealthCause::ContractGap {
            missing: delta.missing.clone(),
            expected: delta.expected,
            actual: delta.actual,
        });
    }

    let (status, healthy) = if healthy_dogs == 0 || !storage_ok {
        ("critical", false)
    } else if healthy_dogs == 1
        || (total_dogs > 0 && healthy_dogs * 2 < total_dogs)
        || probes_degraded
        || !stale_tasks.is_empty()
    {
        ("degraded", false)
    } else if contract_gap {
        // Above operational threshold, but contract not fulfilled.
        // The organism is functional but not fully aware — it's missing senses.
        ("operational", true)
    } else {
        ("sovereign", true)
    };

    HealthAssessment {
        status,
        healthy,
        causes,
    }
}

pub fn system_health_status(
    healthy_dogs: usize,
    total_dogs: usize,
    storage_ok: bool,
    probes_degraded: bool,
    tasks_stale: bool,
) -> (&'static str, bool) {
    let stale_tasks = if tasks_stale {
        vec!["readiness_task"]
    } else {
        vec![]
    };
    let assessment = system_health_assessment(
        healthy_dogs,
        total_dogs,
        storage_ok,
        probes_degraded,
        &stale_tasks,
    );
    (assessment.status, assessment.healthy)
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

    #[test]
    fn assessment_reports_readiness_stale_task_names() {
        let assessment = system_health_assessment(5, 5, true, false, &["health_loop"]);
        assert_eq!(assessment.status, "degraded");
        assert_eq!(
            assessment.causes,
            vec![HealthCause::ReadinessTasksStale {
                tasks: vec!["health_loop"]
            }]
        );
    }

    // ── Contract-aware tests ─────────────────────────────────

    use crate::domain::contract::SystemContract;

    #[test]
    fn contract_fulfilled_stays_sovereign() {
        let contract = SystemContract::new(vec!["a".into(), "b".into()], true);
        let live = vec!["deterministic-dog".into(), "a".into(), "b".into()];
        let delta = contract.assess(&live);
        let assessment =
            system_health_assessment_with_contract(3, 3, true, false, &[], Some(&delta));
        assert_eq!(assessment.status, "sovereign");
        assert!(assessment.healthy);
        assert!(assessment.causes.is_empty());
    }

    #[test]
    fn contract_gap_produces_operational() {
        let contract = SystemContract::new(vec!["qwen".into(), "gemma".into(), "hf".into()], true);
        // gemma missing from live roster
        let live = vec!["deterministic-dog".into(), "qwen".into(), "hf".into()];
        let delta = contract.assess(&live);
        let assessment =
            system_health_assessment_with_contract(3, 3, true, false, &[], Some(&delta));
        assert_eq!(assessment.status, "operational");
        assert!(assessment.healthy); // still functional
        assert!(assessment.causes.iter().any(|c| matches!(
            c,
            HealthCause::ContractGap { missing, .. } if missing.contains(&"gemma".to_string())
        )));
    }

    #[test]
    fn contract_gap_does_not_override_degraded() {
        // degraded conditions take precedence over operational
        let contract = SystemContract::new(vec!["a".into()], true);
        let live = vec!["deterministic-dog".into()]; // missing "a"
        let delta = contract.assess(&live);
        // healthy_dogs=1 → degraded regardless of contract
        let assessment =
            system_health_assessment_with_contract(1, 1, true, false, &[], Some(&delta));
        assert_eq!(assessment.status, "degraded");
        assert!(!assessment.healthy);
    }

    #[test]
    fn no_contract_stays_sovereign() {
        // Backward compat: no contract = old behavior
        let assessment = system_health_assessment_with_contract(3, 3, true, false, &[], None);
        assert_eq!(assessment.status, "sovereign");
    }
}
