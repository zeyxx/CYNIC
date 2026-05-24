//! K15 consumer: polls *-convergence observations, triggers multi-source /judge.
//!
//! Convergence signals are stored by the Python convergence emitter (x/tg organ).
//! This consumer picks them up, builds a rich stimulus, and routes to Dogs.
//!
//! ## Threshold gate (Task 4)
//! Mint-tagged observations (tag: `"mint:<address>"`) are only triggered when
//! `threshold` or more convergence observations exist for that mint within
//! `window_hours`. Non-mint signals are judged immediately (existing behavior).
//!
//! ## Cooldown
//! Mints that have already been triggered are tracked in an in-memory set.
//! After `cooldown_hours` the entry expires and the mint can be triggered again.
//!
//! ## Domain routing
//! Mint-tagged signals → domain `"token-analysis"` (triggers Helius enrichment path).
//! Non-mint signals → original domain from the observation (social stimulus, D1).
//!
//! NOTE: judgment still goes through `judge.evaluate()` which bypasses the full
//! enrichment pipeline. A follow-up task will wire `pipeline::run()` once
//! `PipelineDeps` can be passed to the consumer.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::dog::Stimulus;
use crate::domain::metrics::Metrics;
use crate::domain::slot_semaphore::SlotPriority;
use crate::domain::storage::StoragePort;
use crate::infra::config::ConvergenceConfig;
use crate::judge::Judge;

const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub fn spawn_convergence_consumer(
    judge: Arc<Judge>,
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    config: ConvergenceConfig,
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

        // Track already-judged targets (non-mint): cleared never (session dedup)
        let mut judged_targets: HashSet<String> = HashSet::new();

        // Track already-triggered mints with their trigger timestamp for cooldown.
        // key = mint address, value = Instant when it was last triggered.
        let mut triggered_mints: HashMap<String, Instant> = HashMap::new();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("convergence consumer stopped");
                    break;
                }
                _ = interval.tick() => {
                    run_cycle(&judge, &storage, &metrics, &config, &mut judged_targets, &mut triggered_mints).await;
                }
            }
        }
    })
}

async fn run_cycle(
    judge: &Arc<Judge>,
    storage: &Arc<dyn StoragePort>,
    metrics: &Arc<Metrics>,
    config: &ConvergenceConfig,
    judged_targets: &mut HashSet<String>,
    triggered_mints: &mut HashMap<String, Instant>,
) {
    // Poll convergence-tagged observations directly (not list_observations_raw which
    // buries convergence signals under tool/session observations from active sessions).
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
        let target = obs.target.clone();

        // Determine whether this is a mint-tagged observation.
        if let Some(mint) = extract_mint_from_tags(&obs.tags) {
            // ── Mint path: threshold gate + cooldown + token-analysis routing ──

            let cooldown_duration = Duration::from_secs(config.cooldown_hours * 3600);

            // Cooldown check: skip if recently triggered.
            if let Some(triggered_at) = triggered_mints.get(&mint) {
                if triggered_at.elapsed() < cooldown_duration {
                    tracing::debug!(
                        mint = %mint,
                        elapsed_secs = triggered_at.elapsed().as_secs(),
                        cooldown_secs = cooldown_duration.as_secs(),
                        "convergence: mint in cooldown, skipping"
                    );
                    continue;
                }
                // Cooldown expired — remove so it can be triggered again.
                triggered_mints.remove(&mint);
            }

            // Threshold gate: count recent convergence observations for this mint.
            let mint_tag = format!("mint:{mint}");
            let recent_obs = match tokio::time::timeout(
                Duration::from_secs(10),
                storage.list_observations_by_tag("D1", &mint_tag, 50),
            )
            .await
            {
                Ok(Ok(obs)) => obs,
                Ok(Err(e)) => {
                    tracing::warn!(mint = %mint, "convergence: storage query for mint tag failed: {e}");
                    continue;
                }
                Err(_) => {
                    tracing::warn!(mint = %mint, "convergence: storage query for mint tag timed out");
                    continue;
                }
            };

            // Filter to observations within the rolling window.
            let window_secs = config.window_hours * 3600;
            let count = recent_obs
                .iter()
                .filter(|o| is_within_window(&o.created_at, window_secs))
                .count();

            if count < config.threshold as usize {
                tracing::debug!(
                    mint = %mint,
                    count,
                    threshold = config.threshold,
                    window_hours = config.window_hours,
                    "convergence: below threshold, skipping"
                );
                continue;
            }

            tracing::info!(
                mint = %mint,
                count,
                threshold = config.threshold,
                "convergence_triggered"
            );

            // Route as token-analysis domain — content = raw mint address so the
            // pipeline's enrich_token() path recognises it.
            // TODO: replace judge.evaluate() with pipeline::run() for full Helius
            // enrichment. Currently bypasses the enrichment pipeline. Follow-up task
            // will close this gap once PipelineDeps can be injected here.
            let stimulus = Stimulus {
                content: mint.clone(),
                context: Some(obs.context.clone()),
                domain: Some("token-analysis".to_string()),
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
                        mint = %mint,
                        q_score = format!("{:.3}", verdict.q_score.total),
                        kind = ?verdict.kind,
                        domain = "token-analysis",
                        tags = "auto-convergence",
                        "convergence verdict issued"
                    );
                    triggered_mints.insert(mint, Instant::now());
                    judged_targets.insert(target);
                }
                Ok(Err(e)) => {
                    tracing::warn!(mint = %mint, "convergence judgment failed: {e}");
                }
                Err(_) => {
                    tracing::warn!(mint = %mint, "convergence judgment timed out (120s)");
                }
            }
        } else {
            // ── Non-mint path: original behavior (social stimulus, original domain) ──
            let ctx: serde_json::Value = match serde_json::from_str(&obs.context) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let domain = obs.domain.clone();

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
}

/// Extract a mint address from observation tags.
/// Looks for a tag with the prefix `"mint:"` and returns the address part.
fn extract_mint_from_tags(tags: &[String]) -> Option<String> {
    tags.iter()
        .find_map(|t| t.strip_prefix("mint:"))
        .map(String::from)
}

/// Check whether an ISO-8601 `created_at` timestamp falls within `window_secs` seconds of now.
/// Returns `true` on parse failure (conservative: assume in-window to avoid dropping data).
fn is_within_window(created_at: &str, window_secs: u64) -> bool {
    // Parse RFC3339 / ISO-8601 datetime. SurrealDB stores as strings.
    // Fallback to true (conservative) if parsing fails — avoids silent data loss.
    let Ok(ts) = chrono::DateTime::parse_from_rfc3339(created_at) else {
        return true;
    };
    let now = chrono::Utc::now();
    let age_secs = (now - ts.with_timezone(&chrono::Utc)).num_seconds().max(0) as u64;
    age_secs <= window_secs
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
        let config = ConvergenceConfig::default();
        let shutdown = CancellationToken::new();

        let handle = spawn_convergence_consumer(judge, storage, metrics, config, shutdown.clone());
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(3), handle)
            .await
            .expect("convergence_consumer should stop within 3s")
            .expect("task should not panic");
    }

    #[test]
    fn extract_mint_from_tags_found() {
        let tags = vec![
            "convergence".into(),
            "mint:So11111111111111111111111111111112".into(),
        ];
        assert_eq!(
            extract_mint_from_tags(&tags),
            Some("So11111111111111111111111111111112".into())
        );
    }

    #[test]
    fn extract_mint_from_tags_missing() {
        let tags = vec!["convergence".into(), "social".into()];
        assert_eq!(extract_mint_from_tags(&tags), None);
    }

    #[test]
    fn extract_mint_from_tags_empty() {
        assert_eq!(extract_mint_from_tags(&[]), None);
    }

    #[test]
    fn default_config_values() {
        let c = ConvergenceConfig::default();
        assert_eq!(c.threshold, 3);
        assert_eq!(c.window_hours, 1);
        assert_eq!(c.cooldown_hours, 6);
    }

    #[test]
    fn is_within_window_recent() {
        // A timestamp 30 minutes ago should be within a 1h window.
        let ts = (chrono::Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        assert!(is_within_window(&ts, 3600));
    }

    #[test]
    fn is_within_window_too_old() {
        // A timestamp 2 hours ago should NOT be within a 1h window.
        let ts = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        assert!(!is_within_window(&ts, 3600));
    }

    #[test]
    fn is_within_window_bad_ts_conservative() {
        // Unparseable timestamp → conservatively treat as in-window.
        assert!(is_within_window("not-a-date", 3600));
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
