//! K15 consumer: polls *-convergence observations, triggers multi-source /judge.
//!
//! Convergence signals are stored by the Python convergence emitter (x/tg organ).
//! This consumer picks them up, builds a rich stimulus, and routes to Dogs.
//! Dedup: in-memory set of recently-judged targets (cleared each cycle).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::dog::Stimulus;
use crate::domain::metrics::Metrics;
use crate::domain::slot_semaphore::SlotPriority;
use crate::domain::storage::StoragePort;
use crate::judge::Judge;

const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub fn spawn_convergence_consumer(
    judge: Arc<Judge>,
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Warmup: 30s delay so kernel is fully booted
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(30)) => {}
        }

        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // consume first tick

        // Track already-judged targets to avoid re-processing within a session
        let mut judged_targets: HashSet<String> = HashSet::new();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("convergence consumer stopped");
                    break;
                }
                _ = interval.tick() => {
                    run_cycle(&judge, &storage, &metrics, &mut judged_targets).await;
                }
            }
        }
    })
}

async fn run_cycle(
    judge: &Arc<Judge>,
    storage: &Arc<dyn StoragePort>,
    metrics: &Arc<Metrics>,
    judged_targets: &mut HashSet<String>,
) {
    // Poll convergence-tagged observations directly (not list_observations_raw which
    // buries convergence signals under tool/session observations from active sessions).
    // Query multiple domains — convergence signals come from D1 (default) but also
    // potentially other domains. Use "convergence" tag which the emitter always sets.
    let observations = match tokio::time::timeout(
        Duration::from_secs(10),
        storage.list_observations_by_tag("D1", "convergence", 20),
    )
    .await
    {
        Ok(Ok(obs)) => obs,
        Ok(Err(e)) => {
            tracing::warn!("convergence consumer: storage query failed: {e}");
            return;
        }
        Err(_) => {
            tracing::warn!("convergence consumer: storage query timed out (10s)");
            return;
        }
    };

    let convergence_obs: Vec<_> = observations
        .into_iter()
        .filter(|o| o.tool.ends_with("-convergence"))
        .filter(|o| !judged_targets.contains(&o.target))
        .collect();

    if convergence_obs.is_empty() {
        return;
    }

    tracing::info!(
        count = convergence_obs.len(),
        "convergence consumer: processing signals"
    );

    for obs in convergence_obs {
        // Parse context JSON
        let ctx: serde_json::Value = match serde_json::from_str(&obs.context) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let target = obs.target.clone();
        let domain = obs.domain.clone();

        // Build multi-source stimulus
        let social_section = format_social_section(&ctx);
        let content = format!(
            "[DOMAIN: {domain}]\n\n[SOCIAL SIGNAL — {source}]\n{social}",
            domain = domain,
            source = ctx
                .get("source_organ")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            social = social_section,
        );

        let stimulus = Stimulus {
            content,
            context: Some(obs.context.clone()),
            domain: Some(domain.clone()),
            request_id: None,
        };

        match tokio::time::timeout(
            Duration::from_secs(120),
            judge.evaluate(&stimulus, None, metrics, SlotPriority::Hermes),
        )
        .await
        {
            Ok(Ok(verdict)) => {
                tracing::info!(
                    target = %target,
                    domain = %domain,
                    q_score = format!("{:.3}", verdict.q_score.total),
                    kind = ?verdict.kind,
                    "convergence verdict issued"
                );
                judged_targets.insert(target);
            }
            Ok(Err(e)) => {
                tracing::warn!(target = %target, "convergence judgment failed: {e}");
            }
            Err(_) => {
                tracing::warn!(target = %target, "convergence judgment timed out (120s)");
            }
        }
    }
}

fn format_social_section(ctx: &serde_json::Value) -> String {
    let mut lines = vec![];
    if let Some(count) = ctx.get("author_count").and_then(|v| v.as_u64()) {
        lines.push(format!(
            "convergence: {} authors in {}h",
            count,
            ctx.get("window_hours")
                .and_then(|v| v.as_u64())
                .unwrap_or(6)
        ));
    }
    if let Some(authors) = ctx.get("authors").and_then(|v| v.as_array()) {
        let names: Vec<&str> = authors.iter().filter_map(|a| a.as_str()).collect();
        lines.push(format!("authors: {}", names.join(", ")));
    }
    if let Some(coord) = ctx.get("coordination_score").and_then(|v| v.as_f64()) {
        let label = if coord > 0.5 {
            "coordinated"
        } else {
            "independent"
        };
        lines.push(format!("coordination: {coord:.1} ({label})"));
    }
    if let Some(quotes) = ctx.get("key_quotes").and_then(|v| v.as_array()) {
        lines.push("key_quotes:".to_string());
        for q in quotes.iter().take(3) {
            let author = q.get("author").and_then(|v| v.as_str()).unwrap_or("?");
            let text = q
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .chars()
                .take(120)
                .collect::<String>();
            lines.push(format!("  - @{author}: \"{text}\""));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::NullStorage;

    #[tokio::test]
    async fn convergence_consumer_respects_shutdown() {
        let judge = Arc::new(crate::judge::Judge::new(vec![], vec![]));
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let metrics = Arc::new(Metrics::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_convergence_consumer(judge, storage, metrics, shutdown.clone());
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(3), handle)
            .await
            .expect("convergence_consumer should stop within 3s")
            .expect("task should not panic");
    }

    #[test]
    fn format_social_section_empty_ctx() {
        let ctx = serde_json::json!({});
        let result = format_social_section(&ctx);
        assert!(result.is_empty());
    }

    #[test]
    fn format_social_section_full_ctx() {
        let ctx = serde_json::json!({
            "author_count": 5,
            "window_hours": 6,
            "authors": ["alice", "bob"],
            "coordination_score": 0.8,
            "key_quotes": [
                {"author": "alice", "text": "WAGMI"},
                {"author": "bob", "text": "legit signal"}
            ]
        });
        let result = format_social_section(&ctx);
        assert!(result.contains("convergence: 5 authors in 6h"));
        assert!(result.contains("alice, bob"));
        assert!(result.contains("coordinated"));
        assert!(result.contains("@alice"));
    }
}
