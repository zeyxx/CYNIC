//! Main MAPE-K analyze loop orchestration for introspection.

use std::sync::atomic::Ordering;

use crate::domain::metrics::Metrics;
use crate::domain::organ::OrganSnapshot as SenseSnapshot;
use crate::domain::probe::EnvironmentSnapshot;
use crate::domain::storage::StoragePort;
use crate::introspection::probe::{
    check_resource_thresholds, extract_fleet, extract_pressure, extract_resource,
};
use crate::introspection::rules::{
    analyze_state_log_trends, check_sense_health, is_permanent_source,
};
use crate::introspection::soc::{ForensicsSnapshot, SocThresholds, check_all};
use crate::introspection::types::{Alert, EMBED_FAILURE_THRESHOLD, FAILURE_RATE_THRESHOLD};
use cynic_host::domain::forensics::local::LocalForensicAdapter;
use cynic_host::domain::forensics::port::ForensicObservationPort;
use cynic_protocol::forensics::CrashKind;

/// Run one introspection tick. Returns alerts (empty = healthy).
/// Alerts are logged here and returned to caller. NO pipeline::run calls.
pub async fn analyze(
    storage: &dyn StoragePort,
    metrics: &Metrics,
    environment: &Option<EnvironmentSnapshot>,
    sense_snapshots: &[(String, SenseSnapshot)],
    producer_heartbeats: &std::collections::HashMap<
        String,
        crate::api::rest::types::ProducerHeartbeat,
    >,
) -> Vec<Alert> {
    let mut alerts = Vec::new();

    // ── Dog failure rate ──
    let dog_evals = metrics.dog_evaluations_total.load(Ordering::Relaxed);
    let dog_fails = metrics.dog_failures_total.load(Ordering::Relaxed);
    if dog_evals > 10 {
        let rate = dog_fails as f64 / dog_evals as f64;
        if rate > FAILURE_RATE_THRESHOLD {
            alerts.push(Alert {
                kind: "dog_failure_rate",
                message: format!(
                    "Dog failure rate {:.1}% ({}/{} evals)",
                    rate * 100.0,
                    dog_fails,
                    dog_evals
                ),
                severity: if rate > 0.50 { "critical" } else { "warning" },
            });
        }
    }

    // ── Embedding failure rate ──
    let embed_ok = metrics.embedding_successes_total.load(Ordering::Relaxed);
    let embed_fail = metrics.embedding_failures_total.load(Ordering::Relaxed);
    let embed_total = embed_ok + embed_fail;
    if embed_total > 5 {
        let rate = embed_fail as f64 / embed_total as f64;
        if rate > EMBED_FAILURE_THRESHOLD {
            alerts.push(Alert {
                kind: "embedding_failure_rate",
                message: format!(
                    "Embedding failure rate {:.1}% ({}/{} calls)",
                    rate * 100.0,
                    embed_fail,
                    embed_total
                ),
                severity: "warning",
            });
        }
    }

    // ── Zero verdicts after sustained uptime (cold pipeline) ──
    let verdicts = metrics.verdicts_total.load(Ordering::Relaxed);
    let cache_total = metrics.cache_hits_total.load(Ordering::Relaxed)
        + metrics.cache_misses_total.load(Ordering::Relaxed);
    if verdicts == 0 && cache_total == 0 {
        // No traffic at all — not an anomaly, just idle
    } else if verdicts == 0 && cache_total > 5 {
        alerts.push(Alert {
            kind: "zero_verdicts",
            message: format!(
                "0 verdicts despite {cache_total} cache lookups — pipeline may be broken"
            ),
            severity: "critical",
        });
    }

    // ── Storage health ──
    if storage.ping().await.is_err() {
        alerts.push(Alert {
            kind: "storage_down",
            message: "Storage ping failed — verdicts not persisting".into(),
            severity: "critical",
        });
    }

    // ── Resource metrics (from probe system EnvironmentSnapshot) ──
    if let Some(snap) = environment {
        let resource = extract_resource(snap);
        if let Some(r) = &resource {
            check_resource_thresholds(r, &mut alerts);
        }
        let fleet = extract_fleet(snap);
        if let Some(f) = &fleet {
            for dog in &f.dogs {
                if dog.model_mismatch {
                    alerts.push(Alert {
                        kind: "model_mismatch",
                        message: format!(
                            "Dog '{}' running {:?} but configured as {:?}",
                            dog.dog_name,
                            dog.actual_model.as_deref().unwrap_or("unknown"),
                            dog.expected_model.as_deref().unwrap_or("unknown"),
                        ),
                        severity: "critical",
                    });
                }
            }
        }
        let pressure = extract_pressure(snap);
        if let Some(p) = &pressure {
            let high = p.memory_some_avg10.is_some_and(|v| v > 25.0)
                || p.memory_full_avg10.is_some_and(|v| v > 10.0);
            if high {
                metrics.inc_psi_high_pressure();
                alerts.push(Alert {
                    kind: "psi_memory_pressure",
                    message: format!(
                        "PSI memory pressure: some_avg10={:.1}%, full_avg10={:.1}%",
                        p.memory_some_avg10.unwrap_or(0.0),
                        p.memory_full_avg10.unwrap_or(0.0),
                    ),
                    severity: "warning",
                });
            }
        }
    } else {
        tracing::debug!("introspection: no EnvironmentSnapshot yet — skipping resource checks");
    }

    // ── State log trend detection ──
    match storage.latest_state_blocks(2).await {
        Ok(blocks) if blocks.len() >= 2 => {
            let prev = &blocks[blocks.len() - 2];
            let curr = &blocks[blocks.len() - 1];
            alerts.extend(analyze_state_log_trends(prev, curr));
        }
        Ok(_) => {}
        Err(e) => {
            tracing::debug!("introspection: state_log query failed: {e} — skipping trends");
        }
    }

    // ── Sense metabolism ──
    alerts.extend(check_sense_health(sense_snapshots));

    if !alerts.is_empty() {
        let summary: String = alerts
            .iter()
            .map(|a| format!("[{}] {}", a.severity, a.message))
            .collect::<Vec<_>>()
            .join("; ");
        tracing::warn!(
            alert_count = alerts.len(),
            summary = %summary,
            "introspection: anomalies detected"
        );
    }

    // ── Metabolism: the organism sensing its own data flow ──
    let metabolic_state = crate::domain::metabolism::snapshot(metrics);
    for ma in crate::domain::metabolism::diagnose(&metabolic_state) {
        alerts.push(Alert {
            kind: ma.kind,
            message: ma.message,
            severity: ma.severity,
        });
    }

    // ── Push-producer silence detection ──
    const PRODUCER_SILENCE_SECS: u64 = 600;
    const RETIRED_PRODUCERS: &[&str] = &[
        "hermes-hermes",
        "hermes-x-proxy",
        "hermes-x-organ",
        "x-proxy",
        "hermes-x",
        "hermes",
        "hermes-telegram-personal",
    ];
    for (agent_id, hb) in producer_heartbeats {
        if !is_permanent_source(agent_id) || hb.count < 2 {
            continue;
        }
        if RETIRED_PRODUCERS.contains(&agent_id.as_str()) {
            continue;
        }
        let age_secs = hb.last_seen.elapsed().as_secs();
        if age_secs > PRODUCER_SILENCE_SECS {
            alerts.push(Alert {
                kind: "producer_silent",
                message: format!(
                    "Producer '{}' silent for {}m (was active: {} observations)",
                    agent_id,
                    age_secs / 60,
                    hb.count
                ),
                severity: if age_secs > 3600 {
                    "critical"
                } else {
                    "warning"
                },
            });
        }
    }

    alerts
}

/// Run SOC/forensics detection on host crash evidence.
///
/// Reads forensic evidence from the current boot (GPU faults, kernel traps,
/// service loops), compares against `SocThresholds`, and returns alerts for
/// signals that exceed their thresholds. Emits `KernelEvent::CrashDetected`
/// for confirmed crashes through `event_tx`.
pub fn analyze_soc(
    boot_id: &str,
    event_tx: &tokio::sync::broadcast::Sender<crate::domain::events::KernelEvent>,
) -> Vec<Alert> {
    let adapter = LocalForensicAdapter::new();
    let thresholds = SocThresholds::default();

    // Collect evidence from host
    let evidence = match adapter.collect_evidence(boot_id) {
        Ok(e) => e,
        Err(e) => {
            tracing::debug!(error = %e, "forensics: evidence collection failed");
            return Vec::new();
        }
    };

    // Build forensics snapshot
    let gpu_fault_count = evidence
        .iter()
        .filter(|e| matches!(e.kind, CrashKind::GpuHang))
        .count() as u32;
    let kernel_trap_count = evidence
        .iter()
        .filter(|e| matches!(e.kind, CrashKind::SoftLockup))
        .count() as u32;
    let service_loops = adapter.service_loops().unwrap_or_default();
    let service_restart_rate = service_loops
        .iter()
        .map(|l| l.rate_per_hour())
        .fold(0.0, f64::max);

    let snapshot = ForensicsSnapshot {
        boot_id: boot_id.to_string(),
        gpu_faults: gpu_fault_count,
        kernel_traps: kernel_trap_count,
        service_restart_rate,
        ..Default::default()
    };

    let alerts = check_all(&thresholds, &snapshot);

    // Emit events for confirmed crashes
    for alert in &alerts {
        if alert.severity == "critical" {
            let _ = event_tx.send(crate::domain::events::KernelEvent::CrashDetected {
                kind: alert.kind.to_string(),
                evidence_ref: format!("soc_tick:{}", alert.kind),
                severity: "critical".to_string(),
            });
        }
    }

    alerts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::introspection::soc::SocThresholds;

    #[test]
    fn test_analyze_soc_empty_evidence() {
        let thresholds = SocThresholds::default();
        let snap = ForensicsSnapshot::default();
        let alerts = check_all(&thresholds, &snap);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_analyze_soc_gpu_fault() {
        let thresholds = SocThresholds::default();
        let snap = ForensicsSnapshot {
            gpu_fault_count: 3,
            ..Default::default()
        };
        let alerts = check_all(&thresholds, &snap);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, "gpu_fault");
    }

    #[test]
    fn test_analyze_soc_softlockup() {
        let thresholds = SocThresholds::default();
        let snap = ForensicsSnapshot {
            kernel_trap_count: 2,
            ..Default::default()
        };
        let alerts = check_all(&thresholds, &snap);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, "softlockup");
        assert_eq!(alerts[0].severity, "critical");
    }

    #[test]
    fn test_analyze_soc_multiple() {
        let thresholds = SocThresholds::default();
        let snap = ForensicsSnapshot {
            gpu_fault_count: 1,
            kernel_trap_count: 3,
            service_restart_rate: 600.0,
            ..Default::default()
        };
        let alerts = check_all(&thresholds, &snap);
        assert!(alerts.len() >= 2);
    }
}
