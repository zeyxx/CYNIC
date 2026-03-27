//! MAPE-K Analyze loop — self-observation for ANOMALIES ONLY.
//!
//! Ticks every 5 minutes. Observes internal metrics and emits crystals
//! to domain="cynic-internal" when thresholds are violated.
//! Healthy system = zero observations = silent pipeline.
//!
//! Anti-contamination: skips store_crystal_embedding for internal observations
//! to prevent self-referential noise in the KNN index.

use crate::domain::metrics::Metrics;
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
/// Pure function on metrics snapshot — no side effects except crystal observation.
pub async fn analyze(storage: &dyn StoragePort, metrics: &Metrics) -> Vec<Alert> {
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

    // Emit anomalies as cynic-internal crystal observations (anti-contamination: no embedding)
    if !alerts.is_empty() {
        let now = chrono::Utc::now().to_rfc3339();
        let content = alerts
            .iter()
            .map(|a| format!("[{}] {}", a.severity, a.message))
            .collect::<Vec<_>>()
            .join("; ");
        let id = format!("introspect-{}", &now[..16]); // one per 5-min window
        // Best-effort — don't fail the tick on storage error
        if let Err(e) = storage
            // T8: voter_count=0 — introspection anomalies don't have Dog consensus.
            // This will be rejected by the quorum gate. Introspection crystals
            // need a separate path if we want to persist them without Dog evaluation.
            .observe_crystal(&id, &content, "cynic-internal", 0.3, &now, 0)
            .await
        {
            tracing::warn!(error = %e, "introspection: failed to observe anomaly crystal");
        }
        tracing::warn!(
            alert_count = alerts.len(),
            "introspection: anomalies detected"
        );
    }

    alerts
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
        let alerts = analyze(&NullStorage, &metrics).await;
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
        let alerts = analyze(&NullStorage, &metrics).await;
        assert!(alerts.iter().any(|a| a.kind == "dog_failure_rate"));
    }
}
