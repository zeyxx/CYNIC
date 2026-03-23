//! Judge Pipeline — shared evaluation logic for REST and MCP.
//!
//! This is an application service: it orchestrates domain logic (Judge, CCM)
//! with port calls (StoragePort, EmbeddingPort, VerdictCache).
//! Handlers call this, then format the response for their transport.

use crate::domain::ccm;
use crate::domain::dog::{Stimulus, Verdict, PHI_INV};
use crate::domain::embedding::{Embedding, EmbeddingPort};
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::{CacheLookup, VerdictCache};
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::judge::{Judge, JudgeError};
use tokio::sync::Mutex;

/// Result of the judge pipeline — everything a handler needs to build its response.
pub enum PipelineResult {
    /// Cache hit — near-identical stimulus already judged.
    CacheHit {
        verdict: Box<Verdict>,
        similarity: f64,
    },
    /// Full evaluation — fresh verdict.
    Evaluated {
        verdict: Box<Verdict>,
    },
}

/// Dependencies for the judge pipeline — avoids 9-argument function.
pub struct PipelineDeps<'a> {
    pub judge: &'a Judge,
    pub storage: &'a dyn StoragePort,
    pub embedding: &'a dyn EmbeddingPort,
    pub usage: &'a Mutex<DogUsageTracker>,
    pub verdict_cache: &'a VerdictCache,
    pub metrics: &'a Metrics,
    /// Optional: emit events to SSE/WebSocket subscribers.
    /// None in tests and MCP (no broadcast channel available).
    pub event_tx: Option<&'a tokio::sync::broadcast::Sender<KernelEvent>>,
}

/// Run the full judge pipeline: embed → cache → crystals → sessions → evaluate → store → CCM.
///
/// Both REST and MCP handlers call this. Each formats the response for its transport.
/// All side effects (store, usage, CCM) are best-effort — the verdict is returned
/// even if storage is down.
pub async fn run(
    content: String,
    context: Option<String>,
    domain: Option<String>,
    dogs_filter: Option<&[String]>,
    inject_crystals: bool,
    deps: &PipelineDeps<'_>,
) -> Result<PipelineResult, JudgeError> {
    let PipelineDeps { judge, storage, embedding, verdict_cache, metrics, event_tx, .. } = *deps;
    let domain_hint = domain.as_deref().unwrap_or("general");
    let pipeline_span = tracing::info_span!("judge_pipeline",
        domain = %domain_hint,
        content_len = content.len(),
    );
    let _pipeline_guard = pipeline_span.enter();

    // ── EMBED: stimulus embedding — used for cache AND semantic crystal retrieval ──
    let stimulus_embedding = match embedding.embed(&content).await {
        Ok(emb) => {
            metrics.inc_embed_ok();
            tracing::info!(phase = "embed", dimensions = emb.dimensions, "embedding ok");
            Some(emb)
        }
        Err(e) => {
            metrics.inc_embed_fail();
            tracing::warn!(phase = "embed", error = %e, "embedding failed — cache + crystal merge disabled");
            None
        }
    };

    // ── CACHE CHECK: skip full Dog evaluation if near-identical stimulus exists ──
    // CRITICAL: cache hits still feed the crystal loop. Without this, repeated
    // stimuli (68% of chess, 100% of trading in tests) never accumulate crystal
    // observations → crystals never reach 21 obs → never inject → no compound.
    // Skip cache in A/B mode (crystals=false) — must re-evaluate to measure crystal delta.
    if inject_crystals
        && let Some(emb) = &stimulus_embedding
        && let CacheLookup::Hit { verdict, similarity } = verdict_cache.lookup(emb)
    {
        metrics.inc_cache_hit();
        tracing::info!(phase = "cache", result = "hit", similarity = %format!("{:.4}", similarity),
            verdict_id = %verdict.id, q_score = %format!("{:.3}", verdict.q_score.total));
        observe_crystal_for_verdict(
            &verdict, &stimulus_embedding, domain_hint, deps,
        ).await;
        return Ok(PipelineResult::CacheHit { verdict, similarity });
    }
    metrics.inc_cache_miss();
    tracing::info!(phase = "cache", result = "miss");

    let crystals = if inject_crystals {
        let found = if let Some(ref emb) = stimulus_embedding {
            storage.search_crystals_semantic(&emb.vector, 10).await
                .inspect_err(|e| tracing::warn!(error = %e, "semantic crystal search failed — fallback to domain list"))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if found.is_empty() {
            storage.list_crystals_for_domain(domain_hint, 10).await
                .inspect_err(|e| tracing::warn!(error = %e, "domain crystal list failed — pipeline continues without crystals"))
                .unwrap_or_default()
        } else {
            found
        }
    } else {
        tracing::info!(phase = "crystals", "crystal injection disabled (A/B mode)");
        Vec::new()
    };
    tracing::info!(phase = "crystals", count = crystals.len(), "crystal retrieval complete");

    // ── SESSION SUMMARIES: separate token budget from crystals ──
    let session_ctx = storage.list_session_summaries(5).await
        .inspect_err(|e| tracing::warn!(error = %e, "session summaries failed — pipeline continues without session context"))
        .ok()
        .and_then(|s| ccm::format_session_context(&s, 400));

    // ── CONTEXT ENRICHMENT: merge user context + crystals + sessions ──
    let enriched_context = {
        let crystal_ctx = ccm::format_crystal_context(&crystals, domain_hint, 800);
        let parts: Vec<String> = [context, crystal_ctx, session_ctx]
            .into_iter()
            .flatten()
            .collect();
        if parts.is_empty() { None } else { Some(parts.join("\n\n")) }
    };

    // ── EVALUATE: fan out to Dogs ──
    let stimulus = Stimulus {
        content,
        context: enriched_context,
        domain,
    };
    tracing::info!(phase = "evaluate", "dispatching to Dogs");
    let verdict = judge.evaluate(&stimulus, dogs_filter, metrics).await?;
    metrics.inc_verdict();
    tracing::info!(
        phase = "verdict",
        verdict_id = %verdict.id,
        q_score = %format!("{:.3}", verdict.q_score.total),
        kind = ?verdict.kind,
        dogs_used = %verdict.dog_id,
        failed_dogs = ?verdict.failed_dogs,
        anomaly = verdict.anomaly_detected,
        "verdict issued"
    );

    // ── EMIT EVENT (best-effort — no subscribers = no-op) ──
    if let Some(tx) = event_tx {
        let receivers = tx.receiver_count();
        match tx.send(KernelEvent::VerdictIssued {
            verdict_id: verdict.id.clone(),
            domain: stimulus.domain.as_deref().unwrap_or("general").to_string(),
            verdict: format!("{:?}", verdict.kind),
            q_score: verdict.q_score.total,
        }) {
            Ok(n) => tracing::info!(phase = "event_bus", receivers = n, "verdict event sent"),
            Err(_) => tracing::debug!(phase = "event_bus", receivers = receivers, "no SSE subscribers"),
        }
        // Emit Dog failures as separate events
        for dog_id in &verdict.failed_dogs {
            if tx.send(KernelEvent::DogFailed {
                dog_id: dog_id.clone(),
                error: "evaluation_failed".into(),
            }).is_err() {
                tracing::debug!(phase = "event_bus", dog_id = %dog_id, "no SSE subscribers for DogFailed");
            }
        }
    }

    // ── SIDE EFFECTS (all best-effort) ──
    side_effects(&stimulus, &verdict, &stimulus_embedding, deps).await;

    Ok(PipelineResult::Evaluated { verdict: Box::new(verdict) })
}

/// Best-effort side effects after evaluation.
/// Never fails — all errors are logged and swallowed.
async fn side_effects(
    stimulus: &Stimulus,
    verdict: &Verdict,
    stimulus_embedding: &Option<Embedding>,
    deps: &PipelineDeps<'_>,
) {
    // Store verdict
    if let Err(e) = deps.storage.store_verdict(verdict).await {
        tracing::warn!(phase = "side_effects", error = %e, "failed to store verdict");
    }

    // Track usage
    {
        let mut u = deps.usage.lock().await;
        for ds in &verdict.dog_scores {
            u.record(&ds.dog_id, ds.prompt_tokens, ds.completion_tokens, ds.latency_ms);
        }
        for dog_id in &verdict.failed_dogs {
            u.record_failure(dog_id);
        }
    }

    // CCM: observe crystal + embed
    let domain = stimulus.domain.as_deref().unwrap_or("general");
    observe_crystal_for_verdict(verdict, stimulus_embedding, domain, deps).await;

    // Cache verdict embedding (clone verdict since cache takes ownership of embedding)
    if let Some(emb) = &stimulus_embedding {
        // Clone the embedding for the cache — the original is borrowed
        let cache_emb = Embedding {
            vector: emb.vector.clone(),
            dimensions: emb.dimensions,
            prompt_tokens: emb.prompt_tokens,
        };
        deps.verdict_cache.store(cache_emb, verdict.clone());
    }
}

/// Observe a crystal from a verdict — the critical link in the compound loop.
///
/// Called on BOTH cache hits and full evaluations. Without this on cache hits,
/// repeated stimuli (which are the MOST common in real usage) never accumulate
/// crystal observations → crystals never mature → never inject → no compound.
///
/// Uses semantic merge (cosine ≥ 0.75) to accumulate on existing crystals
/// instead of creating fragmented duplicates.
async fn observe_crystal_for_verdict(
    verdict: &Verdict,
    stimulus_embedding: &Option<Embedding>,
    domain: &str,
    deps: &PipelineDeps<'_>,
) {
    // Semantic merge: find existing crystal or create new via FNV hash
    let crystal_id = if let Some(emb) = stimulus_embedding {
        match deps.storage.find_similar_crystal(&emb.vector, domain, 0.75).await {
            Ok(Some((existing_id, sim))) => {
                tracing::info!(phase = "crystal_merge", crystal_id = %existing_id, similarity = %format!("{:.3}", sim), "reusing existing crystal");
                existing_id
            }
            Ok(None) => {
                tracing::info!(phase = "crystal_merge", "no similar crystal (sim < 0.75), creating new");
                format!("{:x}", ccm::content_hash(&format!("{}:{}", domain, verdict.stimulus_summary)))
            }
            Err(e) => {
                tracing::warn!(phase = "crystal_merge", error = %e, "similarity search failed, using FNV hash");
                format!("{:x}", ccm::content_hash(&format!("{}:{}", domain, verdict.stimulus_summary)))
            }
        }
    } else {
        tracing::info!(phase = "crystal_merge", "no embedding available, using FNV hash");
        format!("{:x}", ccm::content_hash(&format!("{}:{}", domain, verdict.stimulus_summary)))
    };

    let now = chrono::Utc::now().to_rfc3339();
    // Normalize Q-Score: raw scores are φ-bounded (max ≈ 0.618).
    // Without normalization, no crystal can ever reach the 0.618 crystallization threshold.
    let crystal_confidence = (verdict.q_score.total / PHI_INV).min(1.0);
    if let Err(e) = deps.storage.observe_crystal(
        &crystal_id, &verdict.stimulus_summary, domain, crystal_confidence, &now,
    ).await {
        tracing::warn!(phase = "crystal_observe", crystal_id = %crystal_id, error = %e, "failed to observe crystal");
    } else {
        deps.metrics.inc_crystal_obs();
        if let Some(tx) = deps.event_tx {
            let _ = tx.send(KernelEvent::CrystalObserved {
                crystal_id: crystal_id.clone(),
                domain: domain.to_string(),
            }); // ok: no subscribers = silent no-op (receiver_count=0 returns Err)
        }
    }
    if let Some(emb) = stimulus_embedding
        && let Err(e) = deps.storage.store_crystal_embedding(&crystal_id, &emb.vector).await
    {
        tracing::warn!(phase = "crystal_embed", crystal_id = %crystal_id, error = %e, "failed to store crystal embedding");
    }
}

// ── CRYSTAL EMBEDDING BACKFILL ────────────────────────────

/// Backfill embeddings for crystals that were created without one.
/// Crystals without embeddings are permanently invisible to KNN search,
/// meaning they can never be merged or retrieved semantically — orphans forever.
/// Returns the number of crystals successfully embedded.
pub async fn backfill_crystal_embeddings(
    storage: &dyn StoragePort,
    embedding: &dyn EmbeddingPort,
    metrics: &Metrics,
) -> u32 {
    let orphans = match storage.list_crystals_missing_embedding(200).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "backfill: failed to query crystals missing embedding");
            return 0;
        }
    };
    if orphans.is_empty() {
        tracing::info!(phase = "backfill", "no crystals missing embeddings");
        return 0;
    }
    tracing::info!(phase = "backfill", count = orphans.len(), "found crystals missing embeddings");

    let mut success = 0u32;
    let mut failed = 0u32;
    for crystal in &orphans {
        match embedding.embed(&crystal.content).await {
            Ok(emb) => {
                if let Err(e) = storage.store_crystal_embedding(&crystal.id, &emb.vector).await {
                    tracing::warn!(phase = "backfill", crystal_id = %crystal.id, error = %e, "failed to store embedding");
                    failed += 1;
                } else {
                    metrics.inc_embed_ok();
                    success += 1;
                }
            }
            Err(e) => {
                tracing::warn!(phase = "backfill", crystal_id = %crystal.id, error = %e, "embedding failed");
                metrics.inc_embed_fail();
                failed += 1;
            }
        }
    }
    tracing::info!(phase = "backfill", success = success, failed = failed, "backfill complete");
    success
}

// ── SESSION SUMMARIZATION PIPELINE ──────────────────────

/// Summarize sessions that have observations but no summary yet.
/// Takes port traits — testable with NullStorage + NullSummarizer.
/// Returns the number of sessions successfully summarized.
pub async fn summarize_pending_sessions(
    storage: &dyn StoragePort,
    summarizer: &dyn crate::domain::summarization::SummarizationPort,
) -> u32 {
    let pending = match storage.get_unsummarized_sessions(3, 5).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "failed to query unsummarized sessions");
            return 0;
        }
    };

    if pending.is_empty() {
        return 0;
    }

    let mut count = 0u32;
    for (session_id, agent_id, obs_count) in &pending {
        let observations = match storage.get_session_observations(session_id).await {
            Ok(obs) => obs,
            Err(e) => {
                tracing::warn!(session_id = %session_id, error = %e, "failed to get session observations");
                continue;
            }
        };
        if observations.is_empty() {
            continue;
        }

        let prompt = ccm::format_summarization_prompt(&observations);
        let summary_text = match summarizer.summarize(&prompt).await {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!(session_id = %session_id, error = %e, "session summarization failed");
                continue;
            }
        };

        let summary = ccm::SessionSummary {
            session_id: session_id.clone(),
            agent_id: agent_id.clone(),
            summary: summary_text,
            observations_count: *obs_count,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = storage.store_session_summary(&summary).await {
            tracing::warn!(session_id = %session_id, error = %e, "failed to store session summary");
        } else {
            count += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dog::*;
    use crate::domain::health_gate::HealthGate;
    use crate::domain::embedding::NullEmbedding;
    use crate::domain::storage::NullStorage;

    fn test_judge(dogs: Vec<Box<dyn Dog>>) -> Judge {
        let breakers: Vec<std::sync::Arc<dyn HealthGate>> = dogs.iter()
            .map(|d| std::sync::Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(d.id().to_string())) as std::sync::Arc<dyn HealthGate>)
            .collect();
        Judge::new(dogs, breakers)
    }

    #[tokio::test]
    async fn pipeline_runs_with_null_storage_and_null_embedding() {
        // Minimal smoke test: pipeline completes with NullStorage + NullEmbedding
        let dogs: Vec<Box<dyn Dog>> = vec![
            Box::new(crate::dogs::deterministic::DeterministicDog),
        ];
        let judge = test_judge(dogs);
        let storage = NullStorage;
        let embedding = NullEmbedding;
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let metrics = Metrics::new();

        let deps = PipelineDeps {
            judge: &judge, storage: &storage, embedding: &embedding,
            usage: &usage, verdict_cache: &verdict_cache, metrics: &metrics,
            event_tx: None,
        };
        let result = run(
            "1. e4 c5 — The Sicilian Defense".into(),
            None, Some("chess".into()), None, true, &deps,
        ).await;

        match result {
            Ok(PipelineResult::Evaluated { verdict }) => {
                assert!(verdict.q_score.total > 0.0, "Q-Score should be > 0");
                assert!(!verdict.dog_scores.is_empty(), "should have dog scores");
            }
            Ok(PipelineResult::CacheHit { .. }) => panic!("expected evaluation, got cache hit"),
            Err(e) => panic!("pipeline failed: {}", e),
        }
        // Metrics should reflect the pipeline run
        assert_eq!(metrics.verdicts_total.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(metrics.embedding_failures_total.load(std::sync::atomic::Ordering::Relaxed), 1); // NullEmbedding fails
        assert_eq!(metrics.cache_misses_total.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn pipeline_tracks_usage() {
        let dogs: Vec<Box<dyn Dog>> = vec![
            Box::new(crate::dogs::deterministic::DeterministicDog),
        ];
        let judge = test_judge(dogs);
        let storage = NullStorage;
        let embedding = NullEmbedding;
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let metrics = Metrics::new();

        let deps = PipelineDeps {
            judge: &judge, storage: &storage, embedding: &embedding,
            usage: &usage, verdict_cache: &verdict_cache, metrics: &metrics,
            event_tx: None,
        };
        let _ = run("test content".into(), None, None, None, true, &deps).await;

        let u = usage.lock().await;
        assert!(!u.snapshot().is_empty(), "usage should have at least one Dog entry");
    }

    #[tokio::test]
    async fn summarize_pending_sessions_with_null_deps_returns_zero() {
        use crate::domain::summarization::NullSummarizer;
        // NullStorage returns empty vec for get_unsummarized_sessions → 0 sessions to process
        let count = summarize_pending_sessions(&NullStorage, &NullSummarizer).await;
        assert_eq!(count, 0, "NullStorage has no pending sessions");
    }
}
