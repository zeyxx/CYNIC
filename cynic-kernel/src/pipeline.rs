//! Judge Pipeline — shared evaluation logic for REST and MCP.
//!
//! This is an application service: it orchestrates domain logic (Judge, CCM)
//! with port calls (StoragePort, EmbeddingPort, VerdictCache).
//! Handlers call this, then format the response for their transport.

use crate::domain::ccm;
use crate::domain::dog::{PHI_INV, PHI_INV2, PHI_INV3, Stimulus, Verdict};
use crate::domain::embedding::{Embedding, EmbeddingPort};
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::{CacheContext, CacheLookup, VerdictCache};
use crate::judge::{Judge, JudgeError};
use tokio::sync::Mutex;

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

/// Compute epistemic injection weight from Dog disagreement level.
///
/// Returns `(tag, weight)` where tag is "agreed"/"disputed"/"contested"
/// and weight is in [0.0, 1.0]. Pure function — no side effects.
fn epistemic_gate(max_disagreement: f64) -> (&'static str, f64) {
    if max_disagreement < PHI_INV3 {
        ("agreed", 1.0)
    } else if max_disagreement < PHI_INV2 {
        // Linear decay from 1.0 (at φ⁻³) to 0.0 (at φ⁻²)
        let range = PHI_INV2 - PHI_INV3;
        let position = max_disagreement - PHI_INV3;
        ("disputed", (1.0 - position / range).max(0.0))
    } else {
        ("contested", 0.0)
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
///
/// **Epistemic soft gate** (2026-03-24): verdicts with high Dog disagreement
/// are weighted down to prevent crystal poisoning. Three tiers:
///
///   - Agreed   (disagree < φ⁻³): full weight (1.0)
///   - Disputed (φ⁻³ ≤ disagree < φ⁻²): linear decay 1.0→0.0
///   - Contested (disagree ≥ φ⁻²): quarantined — zero crystal update
///
/// Research: noisy-label learning (Northcutt JAIR 2022),
/// recommender feedback bias (FAccT 2024), QBC active learning.
async fn observe_crystal_for_verdict(
    verdict: &Verdict,
    stimulus_embedding: &Option<Embedding>,
    domain: &str,
    deps: &PipelineDeps<'_>,
) {
    // ── T8+T9: Quorum gate — single-Dog verdicts must NOT crystallize ──
    // Verdict is still SERVED (availability), but only consensus crystallizes (integrity).
    if verdict.voter_count < crate::domain::dog::MIN_QUORUM {
        tracing::info!(
            phase = "crystal_gate",
            voter_count = verdict.voter_count,
            min_quorum = crate::domain::dog::MIN_QUORUM,
            "quorum not met — crystal observation skipped (verdict still served)"
        );
        return;
    }

    // ── Epistemic soft gate: weight crystal observation by Dog agreement ──
    let (epistemic_tag, injection_weight) = epistemic_gate(verdict.max_disagreement);

    if injection_weight <= 0.0 {
        tracing::info!(
            phase = "crystal_gate",
            tag = epistemic_tag,
            disagreement = %format!("{:.3}", verdict.max_disagreement),
            "contested verdict — crystal observation quarantined"
        );
        return;
    }

    if injection_weight < 1.0 {
        tracing::info!(
            phase = "crystal_gate",
            tag = epistemic_tag,
            weight = %format!("{:.3}", injection_weight),
            disagreement = %format!("{:.3}", verdict.max_disagreement),
            "disputed verdict — crystal observation weight reduced"
        );
    }
    // Semantic merge: find existing crystal or create new via FNV hash
    let crystal_id = if let Some(emb) = stimulus_embedding {
        match deps
            .storage
            .find_similar_crystal(&emb.vector, domain, 0.75)
            .await
        {
            Ok(Some((existing_id, sim))) => {
                tracing::info!(phase = "crystal_merge", crystal_id = %existing_id, similarity = %format!("{:.3}", sim), "reusing existing crystal");
                existing_id
            }
            Ok(None) => {
                tracing::info!(
                    phase = "crystal_merge",
                    "no similar crystal (sim < 0.75), creating new"
                );
                format!(
                    "{:x}",
                    ccm::content_hash(&ccm::normalize_for_hash(&format!(
                        "{}:{}",
                        domain, verdict.stimulus_summary
                    )))
                )
            }
            Err(e) => {
                tracing::warn!(phase = "crystal_merge", error = %e, "similarity search failed, using FNV hash");
                format!(
                    "{:x}",
                    ccm::content_hash(&ccm::normalize_for_hash(&format!(
                        "{}:{}",
                        domain, verdict.stimulus_summary
                    )))
                )
            }
        }
    } else {
        tracing::info!(
            phase = "crystal_merge",
            "no embedding available, using FNV hash"
        );
        format!(
            "{:x}",
            ccm::content_hash(&ccm::normalize_for_hash(&format!(
                "{}:{}",
                domain, verdict.stimulus_summary
            )))
        )
    };

    let now = chrono::Utc::now().to_rfc3339();
    // Normalize Q-Score: raw scores are φ-bounded (max ≈ 0.618).
    // Without normalization, no crystal can ever reach the 0.618 crystallization threshold.
    // Apply epistemic weight: disputed verdicts contribute less to crystal confidence.
    let crystal_confidence = ((verdict.q_score.total / PHI_INV) * injection_weight).min(1.0);
    if let Err(e) = deps
        .storage
        .observe_crystal(
            &crystal_id,
            &verdict.stimulus_summary,
            domain,
            crystal_confidence,
            &now,
            verdict.voter_count,
            &verdict.id,
        )
        .await
    {
        tracing::warn!(phase = "crystal_observe", crystal_id = %crystal_id, error = %e, "failed to observe crystal");
    } else {
        tracing::info!(
            organ = "crystal",
            action = "observed",
            crystal_id = %crystal_id,
            domain = %domain,
            confidence = %format!("{:.3}", crystal_confidence),
            voters = verdict.voter_count,
            epistemic = %epistemic_tag,
            "crystal observation recorded"
        );
        deps.metrics.inc_crystal_obs();
        if let Some(tx) = deps.event_tx {
            let _ = tx.send(KernelEvent::CrystalObserved {
                crystal_id: crystal_id.clone(),
                domain: domain.to_string(),
            }); // ok: no subscribers = silent no-op (receiver_count=0 returns Err)
        }
    }
    // Store embedding for KNN merge — domain gate above already blocked cynic-internal.
    if let Some(emb) = stimulus_embedding
        && let Err(e) = deps
            .storage
            .store_crystal_embedding(&crystal_id, &emb.vector)
            .await
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

    fn test_judge(dogs: Vec<Box<dyn Dog>>) -> Judge {
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
        let dogs: Vec<Box<dyn Dog>> = vec![Box::new(crate::dogs::deterministic::DeterministicDog)];
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
        let dogs: Vec<Box<dyn Dog>> = vec![Box::new(crate::dogs::deterministic::DeterministicDog)];
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

    // ── Epistemic gate tests ──────────────────────────────────

    #[test]
    fn epistemic_gate_agreed_for_low_disagreement() {
        let (tag, weight) = epistemic_gate(0.1);
        assert_eq!(tag, "agreed");
        assert!((weight - 1.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_agreed_at_zero() {
        let (tag, weight) = epistemic_gate(0.0);
        assert_eq!(tag, "agreed");
        assert!((weight - 1.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_disputed_mid_range() {
        // Midpoint between φ⁻³ (0.236) and φ⁻² (0.382)
        let mid = (PHI_INV3 + PHI_INV2) / 2.0;
        let (tag, weight) = epistemic_gate(mid);
        assert_eq!(tag, "disputed");
        assert!(
            (weight - 0.5).abs() < 0.01,
            "midpoint should be ~0.5, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_disputed_near_agreed_boundary() {
        let (tag, weight) = epistemic_gate(PHI_INV3 + 0.001);
        assert_eq!(tag, "disputed");
        assert!(
            weight > 0.9,
            "near agreed boundary should be ~1.0, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_disputed_near_contested_boundary() {
        let (tag, weight) = epistemic_gate(PHI_INV2 - 0.001);
        assert_eq!(tag, "disputed");
        assert!(
            weight < 0.01,
            "near contested boundary should be ~0.0, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_contested_at_threshold() {
        let (tag, weight) = epistemic_gate(PHI_INV2);
        assert_eq!(tag, "contested");
        assert!((weight - 0.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_contested_above_threshold() {
        let (tag, weight) = epistemic_gate(0.55);
        assert_eq!(tag, "contested");
        assert!((weight - 0.0).abs() < 1e-10);
    }

    // ── crystal_confidence normalization (P1 ACCURACY) ──────

    #[test]
    fn crystal_confidence_howl_reaches_crystallization_threshold() {
        // HOWL verdict: Q-Score total at max φ-bounded = 0.618
        // crystal_confidence = (total / PHI_INV) * weight = (0.618 / 0.618) * 1.0 = 1.0
        let total = PHI_INV; // max possible Q-Score
        let weight = 1.0; // agreed (no dispute)
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(
            conf >= PHI_INV,
            "HOWL verdict should produce crystal_confidence >= φ⁻¹ (crystallization threshold), got {conf}"
        );
    }

    #[test]
    fn crystal_confidence_bark_stays_below_crystallization() {
        // BARK verdict: Q-Score total = 0.2 (well below threshold)
        let total = 0.2;
        let weight = 1.0;
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(
            conf < PHI_INV,
            "BARK verdict should NOT reach crystallization threshold, got {conf}"
        );
    }

    #[test]
    fn crystal_confidence_disputed_howl_reduced() {
        // HOWL Q-Score but disputed (weight < 1.0)
        let total = PHI_INV;
        let weight = 0.5; // disputed midpoint
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(
            conf < PHI_INV,
            "disputed HOWL should be reduced below crystallization, got {conf}"
        );
        assert!(
            conf > 0.0,
            "disputed HOWL should still have positive confidence"
        );
    }

    #[test]
    fn crystal_confidence_clamped_at_one() {
        // Even if formula would exceed 1.0 (shouldn't with phi-bounded inputs, but safety)
        let total = PHI_INV;
        let weight = 1.0;
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(conf <= 1.0, "crystal_confidence must be ≤ 1.0, got {conf}");
    }

    // ── Happy-path pipeline with FixedEmbedding ─────────────

    #[tokio::test]
    async fn pipeline_with_embedding_populates_cache_and_hits() {
        use crate::domain::embedding::FixedEmbedding;
        use crate::storage::memory::InMemoryStorage;

        let dogs: Vec<Box<dyn Dog>> = vec![Box::new(crate::dogs::deterministic::DeterministicDog)];
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
        let dogs: Vec<Box<dyn Dog>> = vec![
            Box::new(crate::dogs::deterministic::DeterministicDog),
            Box::new(FixedDog {
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
