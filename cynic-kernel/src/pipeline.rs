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
    deps: &PipelineDeps<'_>,
) -> Result<PipelineResult, JudgeError> {
    let PipelineDeps { judge, storage, embedding, usage, verdict_cache } = *deps;
    // ── EMBED: stimulus embedding — used for cache AND semantic crystal retrieval ──
    let stimulus_embedding = match embedding.embed(&content).await {
        Ok(emb) => Some(emb),
        Err(e) => {
            eprintln!("[pipeline] Embedding failed (crystal merge + cache disabled): {}", e);
            None
        }
    };

    let domain_hint = domain.as_deref().unwrap_or("general");

    // ── CACHE CHECK: skip full Dog evaluation if near-identical stimulus exists ──
    // CRITICAL: cache hits still feed the crystal loop. Without this, repeated
    // stimuli (68% of chess, 100% of trading in tests) never accumulate crystal
    // observations → crystals never reach 21 obs → never inject → no compound.
    if let Some(emb) = &stimulus_embedding
        && let CacheLookup::Hit { verdict, similarity } = verdict_cache.lookup(emb)
    {
        observe_crystal_for_verdict(
            &verdict, &stimulus_embedding, domain_hint, storage,
        ).await;
        return Ok(PipelineResult::CacheHit { verdict, similarity });
    }
    let crystals = if let Some(ref emb) = stimulus_embedding {
        storage.search_crystals_semantic(&emb.vector, 10).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let crystals = if crystals.is_empty() {
        // Domain-scoped: only mature crystals matching this domain (or "general").
        // Ordered by confidence DESC — the most reliable crystals first.
        storage.list_crystals_for_domain(domain_hint, 10).await.unwrap_or_default()
    } else {
        crystals
    };

    // ── SESSION SUMMARIES: separate token budget from crystals ──
    let session_ctx = storage.list_session_summaries(5).await
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
    let verdict = judge.evaluate(&stimulus, dogs_filter).await?;

    // ── SIDE EFFECTS (all best-effort) ──
    side_effects(&stimulus, &verdict, &stimulus_embedding, storage, usage, verdict_cache).await;

    Ok(PipelineResult::Evaluated { verdict: Box::new(verdict) })
}

/// Best-effort side effects after evaluation.
/// Never fails — all errors are logged and swallowed.
async fn side_effects(
    stimulus: &Stimulus,
    verdict: &Verdict,
    stimulus_embedding: &Option<Embedding>,
    storage: &dyn StoragePort,
    usage: &Mutex<DogUsageTracker>,
    verdict_cache: &VerdictCache,
) {
    // Store verdict
    if let Err(e) = storage.store_verdict(verdict).await {
        eprintln!("[pipeline] Warning: failed to store verdict: {}", e);
    }

    // Track usage
    {
        let mut u = usage.lock().await;
        for ds in &verdict.dog_scores {
            u.record(&ds.dog_id, ds.prompt_tokens, ds.completion_tokens, ds.latency_ms);
        }
        for dog_id in &verdict.failed_dogs {
            u.record_failure(dog_id);
        }
    }

    // CCM: observe crystal + embed
    let domain = stimulus.domain.as_deref().unwrap_or("general");
    observe_crystal_for_verdict(verdict, stimulus_embedding, domain, storage).await;

    // Cache verdict embedding (clone verdict since cache takes ownership of embedding)
    if let Some(emb) = &stimulus_embedding {
        // Clone the embedding for the cache — the original is borrowed
        let cache_emb = Embedding {
            vector: emb.vector.clone(),
            dimensions: emb.dimensions,
            prompt_tokens: emb.prompt_tokens,
        };
        verdict_cache.store(cache_emb, verdict.clone());
    }
}

/// Observe a crystal from a verdict — the critical link in the compound loop.
///
/// Called on BOTH cache hits and full evaluations. Without this on cache hits,
/// repeated stimuli (which are the MOST common in real usage) never accumulate
/// crystal observations → crystals never mature → never inject → no compound.
///
/// Uses semantic merge (cosine ≥ 0.85) to accumulate on existing crystals
/// instead of creating fragmented duplicates.
async fn observe_crystal_for_verdict(
    verdict: &Verdict,
    stimulus_embedding: &Option<Embedding>,
    domain: &str,
    storage: &dyn StoragePort,
) {
    // Semantic merge: find existing crystal or create new via FNV hash
    let crystal_id = if let Some(emb) = stimulus_embedding {
        match storage.find_similar_crystal(&emb.vector, domain, 0.85).await {
            Ok(Some((existing_id, sim))) => {
                eprintln!("[pipeline] Crystal merge: reusing '{}' (similarity {:.3})", existing_id, sim);
                existing_id
            }
            Ok(None) => {
                eprintln!("[pipeline] Crystal fallback: no similar crystal (sim < 0.85), creating new FNV hash");
                format!("{:x}", ccm::content_hash(&format!("{}:{}", domain, verdict.stimulus_summary)))
            }
            Err(e) => {
                eprintln!("[pipeline] Crystal fallback: similarity search failed ({}), using FNV hash", e);
                format!("{:x}", ccm::content_hash(&format!("{}:{}", domain, verdict.stimulus_summary)))
            }
        }
    } else {
        eprintln!("[pipeline] Crystal fallback: no embedding available, using FNV hash");
        format!("{:x}", ccm::content_hash(&format!("{}:{}", domain, verdict.stimulus_summary)))
    };

    let now = chrono::Utc::now().to_rfc3339();
    // Normalize Q-Score: raw scores are φ-bounded (max ≈ 0.618).
    // Without normalization, no crystal can ever reach the 0.618 crystallization threshold.
    let crystal_confidence = (verdict.q_score.total / PHI_INV).min(1.0);
    if let Err(e) = storage.observe_crystal(
        &crystal_id, &verdict.stimulus_summary, domain, crystal_confidence, &now,
    ).await {
        eprintln!("[pipeline] Warning: failed to observe crystal: {}", e);
    }
    if let Some(emb) = stimulus_embedding
        && let Err(e) = storage.store_crystal_embedding(&crystal_id, &emb.vector).await
    {
        eprintln!("[pipeline] Warning: failed to store crystal embedding: {}", e);
    }
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
            eprintln!("[pipeline/summarizer] Failed to query unsummarized sessions: {}", e);
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
                eprintln!("[pipeline/summarizer] Failed to get observations for {}: {}", session_id, e);
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
                eprintln!("[pipeline/summarizer] Failed to summarize session {}: {}", session_id, e);
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
            eprintln!("[pipeline/summarizer] Failed to store summary for {}: {}", session_id, e);
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
    use crate::domain::embedding::NullEmbedding;
    use crate::domain::storage::NullStorage;

    #[tokio::test]
    async fn pipeline_runs_with_null_storage_and_null_embedding() {
        // Minimal smoke test: pipeline completes with NullStorage + NullEmbedding
        let dogs: Vec<Box<dyn Dog>> = vec![
            Box::new(crate::dogs::deterministic::DeterministicDog),
        ];
        let judge = Judge::new(dogs);
        let storage = NullStorage;
        let embedding = NullEmbedding;
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();

        let deps = PipelineDeps {
            judge: &judge, storage: &storage, embedding: &embedding,
            usage: &usage, verdict_cache: &verdict_cache,
        };
        let result = run(
            "1. e4 c5 — The Sicilian Defense".into(),
            None, Some("chess".into()), None, &deps,
        ).await;

        match result {
            Ok(PipelineResult::Evaluated { verdict }) => {
                assert!(verdict.q_score.total > 0.0, "Q-Score should be > 0");
                assert!(!verdict.dog_scores.is_empty(), "should have dog scores");
            }
            Ok(PipelineResult::CacheHit { .. }) => panic!("expected evaluation, got cache hit"),
            Err(e) => panic!("pipeline failed: {}", e),
        }
    }

    #[tokio::test]
    async fn pipeline_tracks_usage() {
        let dogs: Vec<Box<dyn Dog>> = vec![
            Box::new(crate::dogs::deterministic::DeterministicDog),
        ];
        let judge = Judge::new(dogs);
        let storage = NullStorage;
        let embedding = NullEmbedding;
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();

        let deps = PipelineDeps {
            judge: &judge, storage: &storage, embedding: &embedding,
            usage: &usage, verdict_cache: &verdict_cache,
        };
        let _ = run("test content".into(), None, None, None, &deps).await;

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
