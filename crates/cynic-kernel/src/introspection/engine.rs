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
use crate::introspection::types::{Alert, EMBED_FAILURE_THRESHOLD, FAILURE_RATE_THRESHOLD};

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
