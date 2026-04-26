//! MAPE-K Analyze loop — self-observation for ANOMALIES ONLY.
//!
//! Ticks every 5 minutes. Observes internal metrics and returns alerts
//! when thresholds are violated. Healthy system = zero alerts = silent.
//!
//! IMPORTANT: introspection MUST NOT call pipeline::run(). Feeding health
//! alerts through the judge creates a self-amplifying feedback loop:
//! alert → judge → LLM Dogs fail on non-domain text → inc_dog_failure
//! → higher failure rate → more alerts → more judge calls → ...

use crate::domain::metrics::Metrics;
use crate::domain::probe::{EnvironmentSnapshot, FleetDetails, ProbeDetails, ResourceDetails};
use crate::domain::storage::StoragePort;
use std::sync::atomic::Ordering;

/// Anomaly thresholds — conservative defaults.
const FAILURE_RATE_THRESHOLD: f64 = 0.20; // > 20% failure rate
const EMBED_FAILURE_THRESHOLD: f64 = 0.50; // > 50% embedding failures

/// Alert produced by introspection — serialized into /health "alerts" section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Alert {
    pub kind: &'static str,
    pub message: String,
    pub severity: &'static str, // "warning" | "critical"
}

/// Run one introspection tick. Returns alerts (empty = healthy).
/// Alerts are logged here and returned to caller. NO pipeline::run calls.
pub async fn analyze(
    storage: &dyn StoragePort,
    metrics: &Metrics,
    environment: &Option<EnvironmentSnapshot>,
) -> Vec<Alert> {
    let mut alerts = Vec::new();

    // ── Dog failure rate ──
    // dog_evaluations_total = ALL attempts (success + failure).
    // dog_failures_total ⊂ dog_evaluations_total → rate ≤ 1.0 always.
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
        // Fleet: model identity mismatch = wrong model loaded on a backend.
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
    } else {
        tracing::debug!("introspection: no EnvironmentSnapshot yet — skipping resource checks");
    }

    // ── State log trend detection (K15 consumer of state_log) ──
    // Compare last 2 blocks: alert if any Dog's success_rate dropped >20pp.
    // Query all blocks (ASC) and take last 2 for trend comparison.
    // At 60s/block, 1 day = 1440 blocks. Introspection runs every 5min — acceptable query.
    match storage.list_state_blocks("", 1000).await {
        Ok(blocks) if blocks.len() >= 2 => {
            let prev = &blocks[blocks.len() - 2];
            let curr = &blocks[blocks.len() - 1];
            for curr_dog in &curr.dogs {
                if let Some(prev_dog) = prev.dogs.iter().find(|d| d.id == curr_dog.id) {
                    let delta = prev_dog.success_rate - curr_dog.success_rate;
                    if delta > 0.20 {
                        alerts.push(Alert {
                            kind: "dog_degradation_trend",
                            message: format!(
                                "Dog '{}' success rate dropped {:.0}pp ({:.1}% → {:.1}%) in 60s",
                                curr_dog.id,
                                delta * 100.0,
                                prev_dog.success_rate * 100.0,
                                curr_dog.success_rate * 100.0,
                            ),
                            severity: "warning",
                        });
                    }
                }
            }
            // System status regression: sovereign → degraded/critical
            if prev.system.status == "sovereign" && curr.system.status != "sovereign" {
                alerts.push(Alert {
                    kind: "system_regression",
                    message: format!(
                        "System regressed: {} → {} (dogs {}/{} → {}/{})",
                        prev.system.status,
                        curr.system.status,
                        prev.system.healthy_dogs,
                        prev.system.total_dogs,
                        curr.system.healthy_dogs,
                        curr.system.total_dogs,
                    ),
                    severity: "warning",
                });
            }
            // Organ silence: any source silent > 1 hour
            for organ in &curr.organs {
                if organ.silence_secs > 3600 {
                    let hours = organ.silence_secs / 3600;
                    alerts.push(Alert {
                        kind: "organ_silence",
                        message: format!(
                            "Organ '{}' silent for {}h (last obs: {}, total: {})",
                            organ.source, hours, organ.last_observation, organ.total_observations,
                        ),
                        severity: if organ.silence_secs > 86400 {
                            "critical"
                        } else {
                            "warning"
                        },
                    });
                }
            }
        }
        Ok(_) => {} // < 2 blocks, skip trend detection
        Err(e) => {
            tracing::debug!("introspection: state_log query failed: {e} — skipping trends");
        }
    }

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
        // Alerts are returned to caller (spawn_introspection in tasks.rs).
        // They are NOT fed through pipeline::run — that creates a self-amplifying
        // feedback loop where health alerts become stimuli, LLM Dogs fail on them,
        // failure rate increases, generating more alerts. See commit message.
    }

    alerts
}

/// Extract ResourceDetails from an EnvironmentSnapshot's probes.
fn extract_resource(snap: &EnvironmentSnapshot) -> Option<ResourceDetails> {
    snap.probes.iter().find_map(|p| match &p.details {
        ProbeDetails::Resource(r) => Some(r.clone()),
        _ => None,
    })
}

/// Extract FleetDetails from an EnvironmentSnapshot's probes.
fn extract_fleet(snap: &EnvironmentSnapshot) -> Option<FleetDetails> {
    snap.probes.iter().find_map(|p| match &p.details {
        ProbeDetails::Fleet(f) => Some(f.clone()),
        _ => None,
    })
}

/// Check resource thresholds and push alerts.
fn check_resource_thresholds(r: &ResourceDetails, alerts: &mut Vec<Alert>) {
    // Memory pressure
    if let (Some(used), Some(total)) = (r.memory_used_gb, r.memory_total_gb)
        && total > 0.0
    {
        let ratio = used / total;
        if ratio > 0.95 {
            alerts.push(Alert {
                kind: "memory_pressure",
                message: format!(
                    "Memory critical: {:.1}% used ({:.1}/{:.1} GB)",
                    ratio * 100.0,
                    used,
                    total
                ),
                severity: "critical",
            });
        } else if ratio > 0.90 {
            alerts.push(Alert {
                kind: "memory_pressure",
                message: format!(
                    "Memory high: {:.1}% used ({:.1}/{:.1} GB)",
                    ratio * 100.0,
                    used,
                    total
                ),
                severity: "warning",
            });
        }
    }

    // CPU sustained (skip None — first tick or non-Linux)
    if let Some(cpu) = r.cpu_usage_percent
        && cpu > 80.0
    {
        alerts.push(Alert {
            kind: "cpu_sustained",
            message: format!("CPU high: {cpu:.1}%"),
            severity: "warning",
        });
    }

    // Disk low
    if let (Some(avail), Some(total)) = (r.disk_available_gb, r.disk_total_gb)
        && total > 0.0
    {
        let avail_ratio = avail / total;
        if avail_ratio < 0.05 {
            alerts.push(Alert {
                kind: "disk_low",
                message: format!(
                    "Disk critical: {:.1}% available ({:.0}/{:.0} GB)",
                    avail_ratio * 100.0,
                    avail,
                    total
                ),
                severity: "critical",
            });
        } else if avail_ratio < 0.10 {
            alerts.push(Alert {
                kind: "disk_low",
                message: format!(
                    "Disk low: {:.1}% available ({:.0}/{:.0} GB)",
                    avail_ratio * 100.0,
                    avail,
                    total
                ),
                severity: "warning",
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::NullStorage;

    #[tokio::test]
    async fn healthy_system_produces_no_alerts() {
        let metrics = Metrics::new();
        // Simulate some healthy traffic
        metrics.inc_verdict();
        metrics.inc_dog_eval();
        metrics.inc_dog_eval();
        metrics.inc_embed_ok();
        let alerts = analyze(&NullStorage, &metrics, &None).await;
        // NullStorage ping fails → storage_down alert, but no other anomalies
        assert!(alerts.iter().all(|a| a.kind == "storage_down"));
    }

    #[tokio::test]
    async fn high_failure_rate_triggers_alert() {
        let metrics = Metrics::new();
        // 20 total attempts, 10 failures → 50% failure rate
        for _ in 0..20 {
            metrics.inc_dog_eval();
        }
        for _ in 0..10 {
            metrics.inc_dog_failure();
        }
        let alerts = analyze(&NullStorage, &metrics, &None).await;
        assert!(alerts.iter().any(|a| a.kind == "dog_failure_rate"));
    }

    #[tokio::test]
    async fn no_pipeline_run_in_introspection() {
        // This test exists to document and enforce: introspection MUST NOT
        // call pipeline::run(). The function signature proves it — analyze()
        // has no access to Judge, EmbeddingPort, DogUsageTracker, or VerdictCache.
        // If someone re-adds pipeline::run, they'd need to change the signature,
        // which would break this test and all callers.
        let metrics = Metrics::new();
        for _ in 0..20 {
            metrics.inc_dog_eval();
        }
        for _ in 0..15 {
            metrics.inc_dog_failure();
        }
        // Even with 75% failure rate, analyze() just returns alerts — no side effects
        let alerts = analyze(&NullStorage, &metrics, &None).await;
        assert!(alerts.iter().any(|a| a.kind == "dog_failure_rate"));
        // If pipeline::run were called, it would need a Judge (which we don't have).
        // The fact this compiles and runs proves no pipeline call happens.
    }

    #[tokio::test]
    async fn failure_rate_never_exceeds_100_percent() {
        // Regression: previously dog_evaluations_total counted only successes,
        // so rate = failures/successes could exceed 1.0 when failures > successes.
        // Now dog_evaluations_total counts ALL attempts → rate ≤ 1.0 always.
        let metrics = Metrics::new();
        // Simulate: 15 attempts total, 12 of which failed
        for _ in 0..15 {
            metrics.inc_dog_eval(); // all attempts
        }
        for _ in 0..12 {
            metrics.inc_dog_failure(); // just the failures
        }
        let alerts = analyze(&NullStorage, &metrics, &None).await;
        let dog_alert = alerts.iter().find(|a| a.kind == "dog_failure_rate");
        assert!(dog_alert.is_some(), "should trigger failure rate alert");
        // Parse the rate from the message: "Dog failure rate 80.0% (12/15 evals)"
        let msg = &dog_alert.unwrap().message;
        let rate_str = msg
            .split('%')
            .next()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap();
        let rate: f64 = rate_str.parse().unwrap();
        assert!(
            rate <= 100.0,
            "failure rate must never exceed 100%, got {rate}%"
        );
        assert!((rate - 80.0).abs() < 0.1, "expected 80%, got {rate}%");
    }
}
