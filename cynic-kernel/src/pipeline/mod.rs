//! Judge Pipeline — shared evaluation logic for REST and MCP.
//!
//! This is an application service: it orchestrates domain logic (Judge, CCM)
//! with port calls (StoragePort, EmbeddingPort, VerdictCache).
//! Handlers call this, then format the response for their transport.

mod crystal_observer;
mod temporal_eval;

use crate::domain::ccm;
use crate::domain::dog::{Stimulus, Verdict};
use crate::domain::embedding::{Embedding, EmbeddingPort};
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::{CacheContext, CacheLookup, VerdictCache};
use crate::judge::{Judge, JudgeError};

/// Callback type for progressive Dog completion notifications.
pub type OnDogCallback =
    dyn Fn(&str, bool, u64, Option<&crate::domain::dog::DogScore>, Option<String>) + Send + Sync;
use tokio::sync::Mutex;
use tracing::Instrument;

use crystal_observer::observe_crystal_for_verdict;
use temporal_eval::evaluate_temporal;

/// Result of the judge pipeline — everything a handler needs to build its response.
#[derive(Debug)]
pub enum PipelineResult {
    /// Cache hit — near-identical stimulus already judged.
    CacheHit {
        verdict: Box<Verdict>,
        similarity: f64,
    },
    /// Full evaluation — fresh verdict.
    Evaluated { verdict: Box<Verdict> },
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
    /// RC7-2: caller-supplied request_id for cross-subsystem correlation.
    /// If None, pipeline generates one. Propagated to tracing spans + verdict storage.
    pub request_id: Option<String>,
    /// D4: optional progressive callback — called as each Dog completes.
    /// (dog_id, success, elapsed_ms, optional DogScore ref, optional error string)
    pub on_dog: Option<Box<OnDogCallback>>,
}

impl std::fmt::Debug for PipelineDeps<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineDeps").finish_non_exhaustive()
    }
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
    let domain_hint = domain.as_deref().unwrap_or("general");

    // ── Domain gate: reject cynic-internal at pipeline entry ──
    // Self-diagnostic content (Dog failure rates, CPU alerts) must not consume
    // Dog tokens or enter the epistemic pipeline. Block before embedding/cache/eval.
    // See introspection.rs module doc: alerts must not feed back into judgment.
    if domain_hint == "cynic-internal" {
        return Err(JudgeError::InvalidInput(
            "cynic-internal domain is reserved for kernel self-observation and cannot be judged"
                .into(),
        ));
    }

    // RC7-2: use caller-supplied request_id or generate one
    let request_id = deps
        .request_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let pipeline_span = tracing::info_span!("judge_pipeline",
        request_id = %request_id,
        domain = %domain_hint,
        content_len = content.len(),
    );

    // K6 fix: wrap entire future with .instrument() so span propagates through all .awaits.
    // CRITICAL: Do NOT use span.enter() before .await — sync guards don't propagate.
    pipeline_inner(content, context, domain, dogs_filter, inject_crystals, deps)
        .instrument(pipeline_span)
        .await
}

async fn pipeline_inner(
    content: String,
    context: Option<String>,
    domain: Option<String>,
    dogs_filter: Option<&[String]>,
    inject_crystals: bool,
    deps: &PipelineDeps<'_>,
) -> Result<PipelineResult, JudgeError> {
    tracing::info!(content_len = content.len(), "pipeline_inner() started");

    let PipelineDeps {
        judge,
        storage,
        embedding,
        verdict_cache,
        metrics,
        event_tx,
        ..
    } = *deps;
    let domain_hint = domain.as_deref().unwrap_or("general");

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
    // T6: CacheContext ensures domain + Dog config match. A cached chess verdict won't serve
    // a code query. A verdict from 5 Dogs won't serve when only 2 are available.
    let cache_ctx = CacheContext::new(domain_hint, judge.available_dogs_hash(dogs_filter));
    if inject_crystals
        && let Some(emb) = &stimulus_embedding
        && let CacheLookup::Hit {
            verdict,
            similarity,
        } = verdict_cache.lookup(emb, &cache_ctx)
    {
        metrics.inc_cache_hit();
        tracing::info!(phase = "cache", result = "hit", similarity = %format!("{:.4}", similarity),
            verdict_id = %verdict.id, q_score = %format!("{:.3}", verdict.q_score.total));
        observe_crystal_for_verdict(&verdict, &stimulus_embedding, domain_hint, deps).await;
        return Ok(PipelineResult::CacheHit {
            verdict,
            similarity,
        });
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
    // T4: filter to MatureCrystal newtype — only Crystallized|Canonical pass
    let mature_crystals = ccm::filter_mature(crystals);
    tracing::info!(
        phase = "crystals",
        total = mature_crystals.len(),
        "mature crystals for injection"
    );

    // ── SESSION SUMMARIES: separate token budget from crystals ──
    let session_ctx = storage.list_session_summaries(5).await
        .inspect_err(|e| tracing::warn!(error = %e, "session summaries failed — pipeline continues without session context"))
        .ok()
        .and_then(|s| ccm::format_session_context(&s, 400));

    // ── CONTEXT ENRICHMENT: merge user context + crystals + sessions ──
    let enriched_context = {
        // Crystal budget derived from ML theory:
        // - Golden ratio weighting (arXiv 2502.18049): supplementary:primary = φ⁻²
        // - "Lost in the middle" attention research: first 20% of context = high attention
        // - Few-shot ICL research: 3-5 examples optimal
        // All three converge at ~domain_prompt_length × φ⁻² ≈ 1089 for chess (2850 chars).
        // Hardcoded for now; will scale dynamically with domain prompt length.
        const CRYSTAL_BUDGET_CHARS: usize = 1100;
        let crystal_ctx =
            ccm::format_crystal_context(&mature_crystals, domain_hint, CRYSTAL_BUDGET_CHARS);
        let parts: Vec<String> = [context, crystal_ctx, session_ctx]
            .into_iter()
            .flatten()
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    };

    // ── EVALUATE: fan out to Dogs ──
    let stimulus = Stimulus {
        content,
        context: enriched_context,
        domain,
        request_id: deps.request_id.clone(),
    };
    tracing::info!(phase = "evaluate", "dispatching to Dogs");
    let on_dog_ref: Option<&OnDogCallback> = deps.on_dog.as_ref().map(|b| b.as_ref());
    let mut verdict = judge
        .evaluate_progressive(&stimulus, dogs_filter, metrics, on_dog_ref)
        .await?;
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

    // ── K14 GATE: Jury Completeness (missing = degraded) ──
    // Expected Dogs: 4 (deterministic-dog, qwen-7b-hf, qwen35-9b-gpu, gemma-4b-core)
    // If Dogs < expected and not explicitly filtered, downgrade verdict to reflect degraded jury.
    // This prevents silent degradation where missing qwen35 is masked as consensus.
    let expected_dogs = dogs_filter.map(|f| f.len()).unwrap_or(4);
    let actual_dogs = verdict.dog_scores.len();
    if actual_dogs < expected_dogs {
        // K14: Missing Dogs = unreliable jury. Downgrade verdict kind (HOWL → WAG → GROWL).
        use crate::domain::dog::VerdictKind;
        let downgraded = match verdict.kind {
            VerdictKind::Howl => VerdictKind::Wag,
            VerdictKind::Wag => VerdictKind::Growl,
            VerdictKind::Growl => VerdictKind::Bark,
            VerdictKind::Bark => VerdictKind::Bark, // already minimal
        };
        tracing::warn!(
            phase = "jury_gate",
            expected = expected_dogs,
            actual = actual_dogs,
            missing = expected_dogs - actual_dogs,
            original_kind = ?verdict.kind,
            downgraded_kind = ?downgraded,
            "jury incomplete — downgrading verdict per K14 (missing = degraded)"
        );
        verdict.kind = downgraded;
    }

    // ── TEMPORAL PERSPECTIVES (O2: single Dog call for 7 perspectives) ──
    match evaluate_temporal(&stimulus.content) {
        Ok(temporal_verdict) => {
            tracing::info!(
                phase = "temporal",
                perspectives_count = temporal_verdict.perspectives.len(),
                temporal_total = %format!("{:.3}", temporal_verdict.temporal_total),
                outlier = ?temporal_verdict.outlier_perspective,
                divergence = %format!("{:.3}", temporal_verdict.max_divergence),
                "temporal evaluation complete"
            );
        }
        Err(e) => {
            tracing::warn!(phase = "temporal", error = %e, "temporal evaluation failed — continuing without temporal perspective");
        }
    }

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
            Err(_) => tracing::debug!(
                phase = "event_bus",
                receivers = receivers,
                "no SSE subscribers"
            ),
        }
        // Emit Dog failures as separate events
        for dog_id in &verdict.failed_dogs {
            let error_detail = verdict
                .failed_dog_errors
                .get(dog_id)
                .cloned()
                .unwrap_or_else(|| "evaluation_failed".into());
            if tx
                .send(KernelEvent::DogFailed {
                    dog_id: dog_id.clone(),
                    error: error_detail,
                })
                .is_err()
            {
                tracing::debug!(phase = "event_bus", dog_id = %dog_id, "no SSE subscribers for DogFailed");
            }
        }
    }

    // ── SIDE EFFECTS (all best-effort) ──
    side_effects(&stimulus, &verdict, &stimulus_embedding, deps).await;

    Ok(PipelineResult::Evaluated {
        verdict: Box::new(verdict),
    })
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
            u.record(
                &ds.dog_id,
                ds.prompt_tokens,
                ds.completion_tokens,
                ds.latency_ms,
            );
        }
        for dog_id in &verdict.failed_dogs {
            u.record_failure(dog_id);
        }
    }

    // CCM: observe crystal + embed
    let domain = stimulus.domain.as_deref().unwrap_or("general");
    observe_crystal_for_verdict(verdict, stimulus_embedding, domain, deps).await;

    // Cache verdict embedding (clone verdict since cache takes ownership of embedding)
    // T6: store with CacheContext from actual verdict (domain + Dogs that contributed)
    if let Some(emb) = &stimulus_embedding {
        let cache_emb = Embedding {
            vector: emb.vector.clone(),
            dimensions: emb.dimensions,
            prompt_tokens: emb.prompt_tokens,
        };
        // Use verdict.dog_id hash (actual Dogs) for the stored context.
        // At lookup time, Judge::available_dogs_hash() produces the same hash
        // when the same Dogs are available + CB allows them.
        let dog_id_hash = {
            let mut h: u64 = 0xcbf29ce484222325;
            // Sort dog_id segments for deterministic hash (same as Judge::available_dogs_hash)
            let mut ids: Vec<&str> = verdict.dog_id.split('+').collect();
            ids.sort_unstable();
            let joined = ids.join("+");
            for byte in joined.bytes() {
                h ^= byte as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        };
        let store_ctx = CacheContext::new(domain, dog_id_hash);
        deps.verdict_cache
            .store(cache_emb, verdict.clone(), store_ctx);
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
    tracing::info!(
        phase = "backfill",
        count = orphans.len(),
        "found crystals missing embeddings"
    );

    let mut success = 0u32;
    let mut failed = 0u32;
    for crystal in &orphans {
        match embedding.embed(&crystal.content).await {
            Ok(emb) => {
                if let Err(e) = storage
                    .store_crystal_embedding(&crystal.id, &emb.vector)
                    .await
                {
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
        // Rate-limit HNSW index writes: 50ms between each prevents SurrealKV
        // compaction conflicts from burst pressure at boot (176 conflicts/24h root cause).
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    tracing::info!(
        phase = "backfill",
        success = success,
        failed = failed,
        "backfill complete"
    );
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
    use crate::domain::embedding::NullEmbedding;
    use crate::domain::health_gate::HealthGate;
    use crate::domain::storage::NullStorage;
    use std::sync::Arc;

    struct FixedDog {
        name: String,
        scores: AxiomScores,
    }

    #[async_trait::async_trait]
    impl Dog for FixedDog {
        fn id(&self) -> &str {
            &self.name
        }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Ok(self.scores.clone())
        }
    }

    fn test_judge(dogs: Vec<Arc<dyn Dog>>) -> Judge {
        let breakers: Vec<std::sync::Arc<dyn HealthGate>> = dogs
            .iter()
            .map(|d| {
                std::sync::Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
                    d.id().to_string(),
                )) as std::sync::Arc<dyn HealthGate>
            })
            .collect();
        Judge::new(dogs, breakers)
    }

    #[tokio::test]
    async fn pipeline_runs_with_null_storage_and_null_embedding() {
        // Minimal smoke test: pipeline completes with NullStorage + NullEmbedding
        let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
        let judge = test_judge(dogs);
        let storage = NullStorage;
        let embedding = NullEmbedding;
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let metrics = Metrics::new();

        let deps = PipelineDeps {
            judge: &judge,
            storage: &storage,
            embedding: &embedding,
            usage: &usage,
            verdict_cache: &verdict_cache,
            metrics: &metrics,
            event_tx: None,
            request_id: None,
            on_dog: None,
        };

        let result = run(
            "1. e4 c5 — The Sicilian Defense".into(),
            None,
            Some("chess".into()),
            None,
            true,
            &deps,
        )
        .await;

        match result {
            Ok(PipelineResult::Evaluated { verdict }) => {
                assert!(verdict.q_score.total > 0.0, "Q-Score should be > 0");
                assert!(!verdict.dog_scores.is_empty(), "should have dog scores");
            }
            Ok(PipelineResult::CacheHit { .. }) => panic!("expected evaluation, got cache hit"),
            Err(e) => panic!("pipeline failed: {e}"),
        }
        // Metrics should reflect the pipeline run
        assert_eq!(
            metrics
                .verdicts_total
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(
            metrics
                .embedding_failures_total
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        ); // NullEmbedding fails
        assert_eq!(
            metrics
                .cache_misses_total
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn pipeline_tracks_usage() {
        let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
        let judge = test_judge(dogs);
        let storage = NullStorage;
        let embedding = NullEmbedding;
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let metrics = Metrics::new();

        let deps = PipelineDeps {
            judge: &judge,
            storage: &storage,
            embedding: &embedding,
            usage: &usage,
            verdict_cache: &verdict_cache,
            metrics: &metrics,
            event_tx: None,
            request_id: None,
            on_dog: None,
        };
        let _ = run("test content".into(), None, None, None, true, &deps).await;

        let u = usage.lock().await;
        assert!(
            !u.snapshot().is_empty(),
            "usage should have at least one Dog entry"
        );
    }

    #[tokio::test]
    async fn summarize_pending_sessions_with_null_deps_returns_zero() {
        use crate::domain::summarization::NullSummarizer;
        // NullStorage returns empty vec for get_unsummarized_sessions → 0 sessions to process
        let count = summarize_pending_sessions(&NullStorage, &NullSummarizer).await;
        assert_eq!(count, 0, "NullStorage has no pending sessions");
    }

    // ── Happy-path pipeline with FixedEmbedding ─────────────

    #[tokio::test]
    async fn pipeline_with_embedding_populates_cache_and_hits() {
        use crate::domain::embedding::FixedEmbedding;
        use crate::storage::memory::InMemoryStorage;

        let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
        let judge = test_judge(dogs);
        let storage = InMemoryStorage::new();
        // 4-dim unit vector — all stimuli get the same embedding
        let embedding = FixedEmbedding::new(vec![0.5, 0.5, 0.5, 0.5]);
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let metrics = Metrics::new();

        let deps = PipelineDeps {
            judge: &judge,
            storage: &storage,
            embedding: &embedding,
            usage: &usage,
            verdict_cache: &verdict_cache,
            metrics: &metrics,
            event_tx: None,
            request_id: None,
            on_dog: None,
        };

        // First call: should evaluate (cache miss) and embed successfully
        let r1 = run(
            "1. e4 e5 — King's Pawn".into(),
            None,
            Some("chess".into()),
            None,
            true,
            &deps,
        )
        .await
        .unwrap();
        assert!(matches!(r1, PipelineResult::Evaluated { .. }));
        assert_eq!(
            metrics
                .embedding_successes_total
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "embedding should succeed with FixedEmbedding"
        );
        assert_eq!(
            metrics
                .embedding_failures_total
                .load(std::sync::atomic::Ordering::Relaxed),
            0,
            "no embedding failures expected"
        );

        // Second call with same content: should hit cache
        let r2 = run(
            "1. e4 e5 — King's Pawn".into(),
            None,
            Some("chess".into()),
            None,
            true,
            &deps,
        )
        .await
        .unwrap();
        assert!(
            matches!(r2, PipelineResult::CacheHit { similarity, .. } if similarity > 0.99),
            "identical embedding should produce cache hit"
        );
        assert_eq!(
            metrics
                .cache_hits_total
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "should have exactly one cache hit"
        );
    }

    #[tokio::test]
    async fn pipeline_with_embedding_creates_crystal_with_provenance() {
        use crate::domain::embedding::FixedEmbedding;
        use crate::storage::memory::InMemoryStorage;

        // Need 2 Dogs for quorum (MIN_QUORUM = 2)
        let dogs: Vec<Arc<dyn Dog>> = vec![
            Arc::new(crate::dogs::deterministic::DeterministicDog),
            Arc::new(FixedDog {
                name: "quorum-helper".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ];
        let judge = test_judge(dogs);
        let storage = InMemoryStorage::new();
        let embedding = FixedEmbedding::new(vec![0.5, 0.5, 0.5, 0.5]);
        let usage = Mutex::new(DogUsageTracker::new());
        let verdict_cache = VerdictCache::new();
        let metrics = Metrics::new();

        let deps = PipelineDeps {
            judge: &judge,
            storage: &storage,
            embedding: &embedding,
            usage: &usage,
            verdict_cache: &verdict_cache,
            metrics: &metrics,
            event_tx: None,
            request_id: None,
            on_dog: None,
        };

        let result = run(
            "1. e4 c5 — Sicilian Defense".into(),
            None,
            Some("chess".into()),
            None,
            true,
            &deps,
        )
        .await
        .unwrap();

        // Extract verdict ID for provenance check
        let verdict_id = match &result {
            PipelineResult::Evaluated { verdict } => verdict.id.clone(),
            PipelineResult::CacheHit { verdict, .. } => verdict.id.clone(),
        };

        // Crystal should have been created with the verdict's provenance
        let crystals = storage.list_crystals(100).await.unwrap();
        assert!(
            !crystals.is_empty(),
            "pipeline should create at least one crystal"
        );
        let crystal = &crystals[0];
        assert!(
            crystal.contributing_verdicts.contains(&verdict_id),
            "crystal should reference the source verdict (provenance)"
        );
        assert_eq!(crystal.observations, 1);
    }
}
