//! Metabolism — the organism sensing its own data flow.
//!
//! ObservationCounters is computed from Metrics counters (no SQL per tick).
//! It measures intake, digestion, crystallization, and producer health.
//! Gates: introspection uses ObservationCounters to disable dead producers
//! and alert on digestion starvation.

use std::sync::atomic::Ordering;

use crate::domain::metrics::Metrics;

/// φ⁻² — drop rate critical threshold.
const PHI_INV2: f64 = 0.381_966_011_250_105;

/// Point-in-time counters for the observation funnel.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObservationCounters {
    /// Observations accepted by /observe since boot (hydrated from DB).
    pub ingested: u64,
    /// Observations dropped (503 semaphore exhaustion).
    pub dropped: u64,
    /// Observations judged by nightshift Phase 2.
    pub digested: u64,
    /// Nightshift judgment failures.
    pub digestion_errors: u64,
    /// Crystal observations skipped by domain gate ("general" domain).
    pub domain_gate_skipped: u64,
    /// Total verdicts issued.
    pub verdicts: u64,
    /// Total verdicts served to clients (K15 consumption opportunity).
    pub verdicts_served: u64,
    /// Total crystal observations recorded.
    pub crystal_observations: u64,

    // ── Derived ratios (the organism's vital signs) ──
    /// Fraction of accepted observations that have been digested (digested / ingested).
    /// `None` means there is no ingestion denominator yet; unknown is not failure.
    pub observation_digestion_ratio: Option<f64>,
    /// Fraction of issued verdicts served to consumers (verdicts_served / verdicts).
    /// This measures consumption opportunity, not observation digestion.
    /// `None` means no verdict has been issued yet.
    pub verdict_consumption_ratio: Option<f64>,
    /// Fraction of verdicts that produced crystals (crystal_obs / verdicts).
    /// Healthy: > 0.01. Low: < 0.005.
    pub crystallization_rate: f64,
    /// Fraction of observations dropped (dropped / (ingested + dropped)).
    /// Healthy: 0. Any drops = capacity issue.
    pub drop_rate: f64,
    /// Undigested observations (ingested - digested).
    pub backlog: u64,
}

/// Compute ObservationCounters from atomic counters — zero I/O, zero allocation.
pub fn snapshot(metrics: &Metrics) -> ObservationCounters {
    let ingested = metrics.observations_ingested_total.load(Ordering::Relaxed);
    let dropped = metrics.observations_dropped_total.load(Ordering::Relaxed);
    let digested = metrics.nightshift_digested_total.load(Ordering::Relaxed);
    let digestion_errors = metrics.nightshift_errors_total.load(Ordering::Relaxed);
    let domain_gate_skipped = metrics.domain_gate_skipped_total.load(Ordering::Relaxed);
    let verdicts = metrics.verdicts_total.load(Ordering::Relaxed);
    let verdicts_served = metrics.verdicts_served_total.load(Ordering::Relaxed);
    let crystal_observations = metrics.crystal_observations_total.load(Ordering::Relaxed);

    let observation_digestion_ratio = if ingested > 0 {
        Some(digested as f64 / ingested as f64)
    } else {
        None
    };
    let verdict_consumption_ratio = if verdicts > 0 {
        Some(verdicts_served as f64 / verdicts as f64)
    } else {
        None
    };

    let crystallization_rate = if verdicts > 0 {
        crystal_observations as f64 / verdicts as f64
    } else {
        0.0
    };

    let total_attempted = ingested + dropped;
    let drop_rate = if total_attempted > 0 {
        dropped as f64 / total_attempted as f64
    } else {
        0.0
    };

    let backlog = ingested.saturating_sub(digested);

    ObservationCounters {
        ingested,
        dropped,
        digested,
        digestion_errors,
        domain_gate_skipped,
        verdicts,
        verdicts_served,
        crystal_observations,
        observation_digestion_ratio,
        verdict_consumption_ratio,
        crystallization_rate,
        drop_rate,
        backlog,
    }
}

/// Alert emitted by introspection when the funnel's ratios are abnormal.
#[derive(Debug)]
pub struct ObservationCounterAlert {
    pub kind: &'static str,
    pub message: String,
    pub severity: &'static str,
}

/// Diagnose the counters into zero or more alerts.
/// Called by `introspection::analyze()` each tick.
pub fn diagnose(state: &ObservationCounters) -> Vec<ObservationCounterAlert> {
    let mut alerts = Vec::new();

    // Digestion starvation: organism accumulates faster than it processes
    if state.ingested > 100
        && state
            .observation_digestion_ratio
            .is_some_and(|ratio| ratio < 0.10)
    {
        let ratio = state.observation_digestion_ratio.unwrap_or_default();
        alerts.push(ObservationCounterAlert {
            kind: "digestion_starvation",
            message: format!(
                "Digestion ratio {:.1}% — organism accumulates {:.0}x faster than it digests (backlog: {})",
                ratio * 100.0,
                if ratio > 0.0 { 1.0 / ratio } else { f64::INFINITY },
                state.backlog,
            ),
            severity: if ratio < 0.01 { "critical" } else { "warning" },
        });
    }

    // Observation drops: capacity exceeded
    if state.dropped > 0 {
        alerts.push(ObservationCounterAlert {
            kind: "observation_drops",
            message: format!(
                "{} observations dropped ({:.1}% drop rate) — /observe semaphore exhausted",
                state.dropped,
                state.drop_rate * 100.0,
            ),
            severity: if state.drop_rate > PHI_INV2 {
                "critical"
            } else {
                "warning"
            },
        });
    }

    // Low crystallization: verdicts aren't producing wisdom
    if state.verdicts > 50 && state.crystallization_rate < 0.005 {
        alerts.push(ObservationCounterAlert {
            kind: "low_crystallization",
            message: format!(
                "Crystallization rate {:.2}% ({} crystals from {} verdicts) — wisdom extraction stalled",
                state.crystallization_rate * 100.0,
                state.crystal_observations,
                state.verdicts,
            ),
            severity: "warning",
        });
    }

    // Domain gate filtering too aggressively
    if state.verdicts > 50 && state.domain_gate_skipped > state.crystal_observations * 10 {
        alerts.push(ObservationCounterAlert {
            kind: "domain_gate_aggressive",
            message: format!(
                "{} crystal observations skipped by domain gate vs {} recorded — most verdicts produce 'general' domain",
                state.domain_gate_skipped, state.crystal_observations,
            ),
            severity: "warning",
        });
    }

    alerts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_from_fresh_metrics() {
        let m = Metrics::new();
        let s = snapshot(&m);
        assert_eq!(s.ingested, 0);
        assert_eq!(s.backlog, 0);
        assert_eq!(s.observation_digestion_ratio, None);
        assert_eq!(s.verdict_consumption_ratio, None);
    }

    #[test]
    fn digestion_and_consumption_ratios_are_independent() {
        let m = Metrics::new();
        for _ in 0..100 {
            m.inc_observation_ingested();
        }
        m.add_nightshift_digested(25);
        for _ in 0..100 {
            m.inc_verdict();
        }
        for _ in 0..30 {
            m.inc_verdict_served();
        }
        let s = snapshot(&m);
        assert_eq!(s.ingested, 100);
        assert_eq!(s.digested, 25);
        assert_eq!(s.verdicts, 100);
        assert_eq!(s.verdicts_served, 30);
        assert_eq!(s.observation_digestion_ratio, Some(0.25));
        assert_eq!(s.verdict_consumption_ratio, Some(0.3));
    }

    #[test]
    fn starvation_alert_fires() {
        let m = Metrics::new();
        for _ in 0..200 {
            m.inc_observation_ingested();
        }
        m.add_nightshift_digested(5);
        let s = snapshot(&m);
        let alerts = diagnose(&s);
        assert!(
            alerts.iter().any(|a| a.kind == "digestion_starvation"),
            "expected starvation alert at 2.5% digestion"
        );
    }

    #[test]
    fn no_alerts_when_healthy() {
        let m = Metrics::new();
        for _ in 0..100 {
            m.inc_observation_ingested();
        }
        m.add_nightshift_digested(50);
        for _ in 0..20 {
            m.inc_verdict();
        }
        for _ in 0..5 {
            m.inc_crystal_obs();
        }
        let s = snapshot(&m);
        let alerts = diagnose(&s);
        assert!(
            alerts.is_empty(),
            "expected no alerts at 50% digestion, 25% crystallization: {alerts:?}"
        );
    }

    #[test]
    fn drop_alert_fires() {
        let m = Metrics::new();
        m.inc_observation_dropped();
        let s = snapshot(&m);
        let alerts = diagnose(&s);
        assert!(alerts.iter().any(|a| a.kind == "observation_drops"));
    }
}
