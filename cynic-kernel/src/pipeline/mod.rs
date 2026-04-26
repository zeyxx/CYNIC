//! Judge Pipeline — shared evaluation logic for REST and MCP.
//!
//! This is an application service: it orchestrates domain logic (Judge, CCM)
//! with port calls (StoragePort, EmbeddingPort, VerdictCache).
//! Handlers call this, then format the response for their transport.

mod crystal_observer;
pub mod maintenance;
use crate::domain::ccm;
use crate::domain::dog::{Stimulus, Verdict};
use crate::domain::embedding::{Embedding, EmbeddingPort};
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::{CacheContext, CacheLookup, VerdictCache};
use crate::judge::{Judge, JudgeError};
pub use maintenance::{backfill_crystal_embeddings, summarize_pending_sessions};

/// Callback type for progressive Dog completion notifications.
pub type OnDogCallback =
    dyn Fn(&str, bool, u64, Option<&crate::domain::dog::DogScore>, Option<String>) + Send + Sync;
use tokio::sync::Mutex;
use tracing::Instrument;

use crystal_observer::observe_crystal_for_verdict;

/// Result of the judge pipeline — everything a handler needs to build its response.
#[derive(Debug)]
pub enum PipelineResult {
    /// Cache hit — near-identical stimulus already judged.
    CacheHit {
        verdict: Box<Verdict>,
        similarity: f64,
    },
    /// Full evaluation — fresh verdict.
    Evaluated {
        verdict: Box<Verdict>,
        /// Enriched token data, if applicable (domain=token-analysis + Helius).
        token_data: Option<Box<crate::domain::enrichment::TokenData>>,
        /// The actual stimulus content sent to Dogs (may differ from input if enriched).
        enriched_content: Option<String>,
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
    /// RC7-2: caller-supplied request_id for cross-subsystem correlation.
    /// If None, pipeline generates one. Propagated to tracing spans + verdict storage.
    pub request_id: Option<String>,
    /// D4: optional progressive callback — called as each Dog completes.
    /// (dog_id, success, elapsed_ms, optional DogScore ref, optional error string)
    pub on_dog: Option<Box<OnDogCallback>>,
    /// Expected fleet size — drives K14 jury gate.
    /// Comes from `judge.dog_ids().len()`, NOT hardcoded.
    pub expected_dog_count: usize,
    /// Optional token enricher — when domain=token-analysis and content is a Solana address,
    /// enriches the stimulus with on-chain data before Dogs evaluate.
    pub enricher: Option<&'a dyn crate::domain::enrichment::TokenEnricherPort>,
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

    // ── WALLET JUDGMENT: save raw context before enrichment merges it ──
    let wallet_context_json = if domain_hint == "wallet-judgment" {
        context.clone()
    } else {
        None
    };

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

    // ── TOKEN ENRICHMENT: rewrite stimulus with on-chain data when applicable ──
    let mut captured_token_data: Option<crate::domain::enrichment::TokenData> = None;
    let content = if domain_hint == "token-analysis"
        && crate::domain::enrichment::looks_like_solana_address(&content)
    {
        if let Some(enricher) = deps.enricher {
            match tokio::time::timeout(
                std::time::Duration::from_secs(12),
                enricher.enrich(&content),
            )
            .await
            {
                Ok(Ok(Some(token_data))) => {
                    let enriched = token_data.to_stimulus();
                    tracing::info!(
                        phase = "enrich",
                        mint = %content,
                        name = ?token_data.name,
                        symbol = ?token_data.symbol,
                        holders = ?token_data.holder_count,
                        "token enriched via Helius"
                    );
                    captured_token_data = Some(token_data);
                    enriched
                }
                Ok(Ok(None)) => {
                    tracing::warn!(phase = "enrich", mint = %content, "not a recognized token — using raw address");
                    content
                }
                Ok(Err(e)) => {
                    tracing::warn!(phase = "enrich", error = %e, "enrichment failed — using raw address");
                    content
                }
                Err(_) => {
                    tracing::warn!(
                        phase = "enrich",
                        "enrichment timed out (12s) — using raw address"
                    );
                    content
                }
            }
        } else {
            tracing::debug!(
                phase = "enrich",
                "no enricher configured — using raw address"
            );
            content
        }
    } else {
        content
    };
    let enriched_content = if captured_token_data.is_some() {
        Some(content.clone())
    } else {
        None
    };

    // ── WALLET ENRICHMENT: rewrite content with structured wallet metrics ──
    let mut captured_wallet_profile: Option<crate::domain::wallet_judgment::WalletProfile> = None;
    let content = if domain_hint == "wallet-judgment" {
        match wallet_context_json.as_deref().and_then(|ctx| {
            serde_json::from_str::<crate::domain::wallet_judgment::WalletProfile>(ctx).ok()
        }) {
            Some(profile) => {
                tracing::info!(
                    phase = "enrich",
                    wallet = %profile.wallet_address,
                    games = profile.games_completed,
                    age_days = profile.wallet_age_days,
                    "wallet profile parsed"
                );
                let stimulus = profile.to_stimulus();
                captured_wallet_profile = Some(profile);
                stimulus
            }
            None => {
                tracing::warn!(
                    phase = "enrich",
                    "wallet-judgment: no valid WalletProfile JSON in context — using raw content"
                );
                content
            }
        }
    } else {
        content
    };

    // ── EVALUATE: fan out to Dogs ──
    let stimulus = Stimulus {
        content,
        context: enriched_context,
        domain: domain.clone(),
        request_id: deps.request_id.clone(),
    };

    // ── WALLET-JUDGMENT: fast-path deterministic verdict (no LLM Dogs) ──
    if domain_hint == "wallet-judgment" {
        let verdict = if let Some(ref profile) = captured_wallet_profile {
            use crate::domain::dog::{DogScore, Verdict, compute_qscore, phi_bound, verdict_kind};
            let (_, axiom_scores) = crate::domain::wallet_judgment::deterministic_dog(profile);
            let q_score = compute_qscore(&axiom_scores);
            let kind = verdict_kind(q_score.total);
            let id = uuid::Uuid::new_v4().to_string();
            let timestamp = chrono::Utc::now().to_rfc3339();
            let stimulus_summary: String = stimulus.content.chars().take(100).collect();
            let dog_score = DogScore {
                dog_id: "wallet-deterministic-dog".to_string(),
                latency_ms: 0,
                prompt_tokens: 0,
                completion_tokens: 0,
                fidelity: phi_bound(axiom_scores.fidelity),
                phi: phi_bound(axiom_scores.phi),
                verify: phi_bound(axiom_scores.verify),
                culture: phi_bound(axiom_scores.culture),
                burn: phi_bound(axiom_scores.burn),
                sovereignty: phi_bound(axiom_scores.sovereignty),
                raw_fidelity: axiom_scores.fidelity,
                raw_phi: axiom_scores.phi,
                raw_verify: axiom_scores.verify,
                raw_culture: axiom_scores.culture,
                raw_burn: axiom_scores.burn,
                raw_sovereignty: axiom_scores.sovereignty,
                reasoning: axiom_scores.reasoning.clone(),
                abstentions: axiom_scores.abstentions.clone(),
            };
            Verdict {
                id,
                domain: "wallet-judgment".to_string(),
                kind,
                q_score,
                reasoning: axiom_scores.reasoning,
                dog_id: "wallet-deterministic-dog".to_string(),
                stimulus_summary,
                timestamp,
                voter_count: 1,
                dog_scores: vec![dog_score],
                anomaly_detected: false,
                max_disagreement: 0.0,
                anomaly_axiom: None,
                failed_dogs: vec![],
                failed_dog_errors: std::collections::HashMap::new(),
                integrity_hash: None, // MVP: wallet verdicts start fresh chain post-hackathon
                prev_hash: None,
            }
        } else {
            return Err(crate::judge::JudgeError::InvalidInput(
                "wallet-judgment requires valid WalletProfile JSON in context field".to_string(),
            ));
        };

        tracing::info!(
            phase = "verdict",
            verdict_id = %verdict.id,
            q_score = %format!("{:.3}", verdict.q_score.total),
            kind = ?verdict.kind,
            "wallet verdict issued (deterministic)"
        );

        // Event emission (best-effort)
        if let Some(tx) = event_tx {
            let _ = tx.send(KernelEvent::VerdictIssued {
                verdict_id: verdict.id.clone(),
                domain: "wallet-judgment".to_string(),
                verdict: format!("{:?}", verdict.kind),
                q_score: verdict.q_score.total,
            });
        }

        // Side effects (store + usage tracking + CCM)
        side_effects(&stimulus, &verdict, &stimulus_embedding, deps).await;

        return Ok(PipelineResult::Evaluated {
            verdict: Box::new(verdict),
            token_data: None,
            enriched_content: None,
        });
    }

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
    // If Dogs < expected and not explicitly filtered, downgrade verdict to reflect degraded jury.
    // This prevents silent degradation where missing Dogs are masked as consensus.
    let expected_dogs = dogs_filter
        .map(|f| f.len())
        .unwrap_or(deps.expected_dog_count);
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
        token_data: captured_token_data.map(Box::new),
        enriched_content,
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
            expected_dog_count: judge.dog_ids().len(),
            enricher: None,
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
            Ok(PipelineResult::Evaluated { verdict, .. }) => {
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
            expected_dog_count: judge.dog_ids().len(),
            enricher: None,
        };
        let _ = run("test content".into(), None, None, None, true, &deps).await;

        let u = usage.lock().await;
        assert!(
            !u.snapshot().is_empty(),
            "usage should have at least one Dog entry"
        );
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
            expected_dog_count: judge.dog_ids().len(),
            enricher: None,
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
            expected_dog_count: judge.dog_ids().len(),
            enricher: None,
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
            PipelineResult::Evaluated { verdict, .. } => verdict.id.clone(),
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

    #[tokio::test]
    async fn wallet_judgment_returns_verdict_from_deterministic_dog() {
        use crate::domain::embedding::FixedEmbedding;
        use crate::domain::wallet_judgment::WalletProfile;
        use crate::storage::memory::InMemoryStorage;

        // Build a valid WalletProfile with 5+ games
        let profile = WalletProfile {
            wallet_address: "TestWallet1234567890".to_string(),
            games_completed: 10,
            archetype_consistency: 0.80,
            wallet_age_days: 30,
            average_game_duration: 300,
            duration_variance: 0.20,
            opening_repertoire_hash: "hash1".to_string(),
            move_sequence_hash: "hash2".to_string(),
            suspicious_cluster: false,
            replay_risk: false,
        };
        let ctx = serde_json::to_string(&profile).unwrap();

        // Set up pipeline dependencies
        let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
        let judge = test_judge(dogs);
        let storage = InMemoryStorage::new();
        let embedding = FixedEmbedding::new(vec![0.5; 4]);
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
            expected_dog_count: judge.dog_ids().len(),
            enricher: None,
        };

        let result = run(
            profile.wallet_address.clone(),
            Some(ctx),
            Some("wallet-judgment".into()),
            None,
            true,
            &deps,
        )
        .await
        .unwrap();

        // Assert verdict returned with q_score > 0 and dog_id = "wallet-deterministic-dog"
        match result {
            PipelineResult::Evaluated { verdict, .. } => {
                assert_eq!(verdict.dog_id, "wallet-deterministic-dog");
                assert!(
                    verdict.q_score.total > 0.0,
                    "wallet verdict should have positive q_score"
                );
                assert_eq!(verdict.domain, "wallet-judgment");
            }
            _ => panic!("Expected Evaluated result"),
        }
    }

    #[tokio::test]
    async fn wallet_judgment_bark_on_insufficient_games() {
        use crate::domain::embedding::FixedEmbedding;
        use crate::domain::wallet_judgment::WalletProfile;
        use crate::storage::memory::InMemoryStorage;

        // Create profile with only 3 games (below gate threshold of 5)
        let profile = WalletProfile {
            wallet_address: "InsufficientGames".to_string(),
            games_completed: 3,
            archetype_consistency: 0.80,
            wallet_age_days: 30,
            average_game_duration: 300,
            duration_variance: 0.20,
            opening_repertoire_hash: "hash1".to_string(),
            move_sequence_hash: "hash2".to_string(),
            suspicious_cluster: false,
            replay_risk: false,
        };
        let ctx = serde_json::to_string(&profile).unwrap();

        // Set up pipeline
        let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
        let judge = test_judge(dogs);
        let storage = InMemoryStorage::new();
        let embedding = FixedEmbedding::new(vec![0.5; 4]);
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
            expected_dog_count: judge.dog_ids().len(),
            enricher: None,
        };

        let result = run(
            profile.wallet_address.clone(),
            Some(ctx),
            Some("wallet-judgment".into()),
            None,
            true,
            &deps,
        )
        .await
        .unwrap();

        // Assert BARK verdict
        match result {
            PipelineResult::Evaluated { verdict, .. } => {
                assert_eq!(
                    verdict.kind,
                    crate::domain::dog::VerdictKind::Bark,
                    "insufficient games should produce BARK verdict"
                );
            }
            _ => panic!("Expected Evaluated result"),
        }
    }

    #[tokio::test]
    async fn wallet_judgment_error_on_missing_profile() {
        use crate::domain::embedding::FixedEmbedding;
        use crate::storage::memory::InMemoryStorage;

        // Call with domain="wallet-judgment" but no/invalid context
        let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
        let judge = test_judge(dogs);
        let storage = InMemoryStorage::new();
        let embedding = FixedEmbedding::new(vec![0.5; 4]);
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
            expected_dog_count: judge.dog_ids().len(),
            enricher: None,
        };

        let result = run(
            "TestWallet".into(),
            None, // No context provided
            Some("wallet-judgment".into()),
            None,
            true,
            &deps,
        )
        .await;

        // Assert JudgeError::InvalidInput returned
        assert!(
            result.is_err(),
            "wallet-judgment without valid profile should error"
        );
        match result {
            Err(e) => {
                // Verify it's an InvalidInput error
                assert!(
                    format!("{e:?}").contains("InvalidInput"),
                    "should be InvalidInput error"
                );
            }
            _ => panic!("Expected error result"),
        }
    }
}
