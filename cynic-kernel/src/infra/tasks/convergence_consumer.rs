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
//! Mint-tagged signals → `pipeline::run()` with domain `"token-analysis"`.
//!   Full enrichment: Helius + DexScreener + rug prefilter + rug risk + crystals + Dogs.
//! Non-mint signals → `judge.evaluate()` with original domain (social stimulus, D1).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::dog::Stimulus;
use crate::domain::metrics::Metrics;
use crate::domain::slot_semaphore::SlotPriority;
use crate::domain::storage::StoragePort;
use crate::infra::config::ConvergenceConfig;
use crate::judge::Judge;
use crate::pipeline::PipelineDeps;

/// Pipeline-related deps for mint-tagged convergence signals.
/// Enables full enrichment (Helius, rug risk, crystals) via `pipeline::run()`.
pub struct ConvergencePipelineDeps {
    pub embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
    pub usage: Arc<Mutex<crate::domain::usage::DogUsageTracker>>,
    pub verdict_cache: Arc<crate::domain::verdict_cache::VerdictCache>,
    pub enricher: Option<Arc<dyn crate::domain::enrichment::TokenEnricherPort>>,
    pub domain_curations: Arc<crate::domain::wisdom::DomainCurations>,
    pub domain_router: Arc<crate::infra::domain_router::DomainRouter>,
    pub routing_calc: Arc<crate::infra::routing_calc::RoutingCalculator>,
    pub event_tx: Option<tokio::sync::broadcast::Sender<crate::domain::events::KernelEvent>>,
}

impl std::fmt::Debug for ConvergencePipelineDeps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConvergencePipelineDeps")
            .field("enricher", &self.enricher.is_some())
            .finish_non_exhaustive()
    }
}

const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub fn spawn_convergence_consumer(
    judge: Arc<Judge>,
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    config: ConvergenceConfig,
    pipeline: ConvergencePipelineDeps,
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
                    run_cycle(&judge, &storage, &metrics, &config, &pipeline, &mut judged_targets, &mut triggered_mints).await;
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
    pipeline: &ConvergencePipelineDeps,
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

            // K25: rate-limit between iterations to avoid starving interactive slots.
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Full pipeline: embed → cache → crystals → Helius enrichment → rug
            // prefilter → rug risk scoring → Dogs → side effects (store + CCM).
            let deps = PipelineDeps {
                judge,
                storage: storage.as_ref(),
                embedding: pipeline.embedding.as_ref(),
                usage: &pipeline.usage,
                verdict_cache: &pipeline.verdict_cache,
                metrics,
                event_tx: pipeline.event_tx.as_ref(),
                request_id: None,
                on_dog: None,
                expected_dog_count: judge.dog_ids().len(),
                enricher: pipeline.enricher.as_deref(),
                domain_curations: &pipeline.domain_curations,
                domain_router: Some(&pipeline.domain_router),
                routing_calc: Some(&pipeline.routing_calc),
                priority: SlotPriority::Hermes,
            };

            match tokio::time::timeout(
                Duration::from_secs(120),
                crate::pipeline::run(
                    mint.clone(),
                    Some(obs.context.clone()),
                    Some("token-analysis".to_string()),
                    None, // no dogs filter
                    true, // inject crystals
                    &deps,
                ),
            )
            .await
            {
                Ok(Ok(result)) => {
                    let (q_score, kind) = match &result {
                        crate::pipeline::PipelineResult::Evaluated { verdict, .. } => (
                            format!("{:.3}", verdict.q_score.total),
                            format!("{:?}", verdict.kind),
                        ),
                        crate::pipeline::PipelineResult::CacheHit { verdict, .. } => (
                            format!("{:.3}", verdict.q_score.total),
                            format!("{:?}", verdict.kind),
                        ),
                    };
                    tracing::info!(
                        mint = %mint,
                        q_score = %q_score,
                        kind = %kind,
                        domain = "token-analysis",
                        tags = "auto-convergence",
                        "convergence verdict issued (enriched pipeline)"
                    );
                    triggered_mints.insert(mint, Instant::now());
                    judged_targets.insert(target);
                }
                Ok(Err(e)) => {
                    tracing::warn!(mint = %mint, "convergence pipeline failed: {e}");
                }
                Err(_) => {
                    tracing::warn!(mint = %mint, "convergence pipeline timed out (120s)");
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

        let pipeline = ConvergencePipelineDeps {
            embedding: Arc::new(crate::domain::embedding::NullEmbedding),
            usage: Arc::new(Mutex::new(crate::domain::usage::DogUsageTracker::new())),
            verdict_cache: Arc::new(crate::domain::verdict_cache::VerdictCache::new()),
            enricher: None,
            domain_curations: Arc::new(crate::domain::wisdom::DomainCurations::new()),
            domain_router: Arc::new(crate::infra::domain_router::DomainRouter::from_backends(&[])),
            routing_calc: Arc::new(crate::infra::routing_calc::RoutingCalculator::new()),
            event_tx: None,
        };

        let handle =
            spawn_convergence_consumer(judge, storage, metrics, config, pipeline, shutdown.clone());
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
