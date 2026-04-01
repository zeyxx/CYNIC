//! MAPE-K Analyze loop — self-observation for ANOMALIES ONLY.
//!
//! Ticks every 5 minutes. Observes internal metrics and emits crystals
//! to domain="cynic-internal" when thresholds are violated.
//! Healthy system = zero observations = silent pipeline.
//!
//! Anti-contamination: skips store_crystal_embedding for internal observations
//! to prevent self-referential noise in the KNN index.

use crate::domain::embedding::EmbeddingPort;
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::domain::probe::{EnvironmentSnapshot, ProbeDetails, ResourceDetails};
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::VerdictCache;
use crate::judge::Judge;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;

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
/// Receives individual dependencies (not PipelineDeps — borrows can't cross tokio::spawn).
#[allow(clippy::too_many_arguments)]
// WHY: analyze() is the single MAPE-K analysis entry point — each argument is an independent
// kernel dependency that cannot be merged without coupling unrelated subsystems. A builder or
// context struct would just rename the problem without reducing actual coupling.
pub async fn analyze(
    storage: &dyn StoragePort,
    metrics: &Metrics,
    environment: &Option<EnvironmentSnapshot>,
    judge: &Judge,
    embedding: &dyn EmbeddingPort,
    usage: &Mutex<DogUsageTracker>,
    verdict_cache: &VerdictCache,
    event_tx: Option<&tokio::sync::broadcast::Sender<KernelEvent>>,
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
    } else {
        tracing::debug!("introspection: no EnvironmentSnapshot yet — skipping resource checks");
    }

    if !alerts.is_empty() {
        tracing::warn!(
            alert_count = alerts.len(),
            "introspection: anomalies detected"
        );

        // Consolidate all alerts into ONE pipeline::run call (storm mitigation)
        let content: String = alerts
            .iter()
            .map(|a| format!("[{}] {}", a.severity, a.message))
            .collect::<Vec<_>>()
            .join("; ");
        // Bound to 2000 chars
        let content: String = content.chars().take(2000).collect();

        // Construct PipelineDeps from borrowed references
        let deps = crate::pipeline::PipelineDeps {
            judge,
            storage,
            embedding,
            usage,
            verdict_cache,
            metrics,
            event_tx,
            request_id: None,
        };

        let dogs: Vec<String> = vec!["deterministic-dog".into(), "gemini-flash".into()];
        match crate::pipeline::run(
            content,
            None,
            Some("cynic-internal".to_string()),
            Some(dogs.as_slice()),
            false,
            &deps,
        )
        .await
        {
            Ok(_result) => {
                tracing::info!("introspection: anomaly submitted to pipeline for crystallization");
            }
            Err(e) => {
                tracing::warn!(error = %e, "introspection: pipeline::run failed for anomaly");
            }
        }
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
    use crate::domain::embedding::NullEmbedding;
    use crate::domain::storage::NullStorage;
    use crate::domain::verdict_cache::VerdictCache;
    use crate::judge::Judge;

    fn make_test_judge() -> Judge {
        // Zero Dogs — evaluate returns empty, voter_count=0
        Judge::new(vec![], vec![])
    }

    #[tokio::test]
    async fn healthy_system_produces_no_alerts() {
        let metrics = Metrics::new();
        // Simulate some healthy traffic
        metrics.inc_verdict();
        metrics.inc_dog_eval();
        metrics.inc_dog_eval();
        metrics.inc_embed_ok();
        let judge = make_test_judge();
        let embedding = NullEmbedding;
        let usage = Mutex::new(crate::domain::usage::DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let alerts = analyze(
            &NullStorage,
            &metrics,
            &None,
            &judge,
            &embedding,
            &usage,
            &verdict_cache,
            None,
        )
        .await;
        // NullStorage ping fails → storage_down alert, but no other anomalies
        assert!(alerts.iter().all(|a| a.kind == "storage_down"));
    }

    #[tokio::test]
    async fn high_failure_rate_triggers_alert() {
        let metrics = Metrics::new();
        for _ in 0..20 {
            metrics.inc_dog_eval();
        }
        for _ in 0..10 {
            metrics.inc_dog_failure();
        }
        let judge = make_test_judge();
        let embedding = NullEmbedding;
        let usage = Mutex::new(crate::domain::usage::DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let alerts = analyze(
            &NullStorage,
            &metrics,
            &None,
            &judge,
            &embedding,
            &usage,
            &verdict_cache,
            None,
        )
        .await;
        assert!(alerts.iter().any(|a| a.kind == "dog_failure_rate"));
    }
}
