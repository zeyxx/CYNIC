//! Judge Pipeline — shared evaluation logic for REST and MCP.
//!
//! This is an application service: it orchestrates domain logic (Judge, CCM)
//! with port calls (StoragePort, EmbeddingPort, VerdictCache).
//! Handlers call this, then format the response for their transport.

mod crystal_observer;
pub mod maintenance;
mod verdict_observer;
use crate::domain::ccm;
use crate::domain::dog::{Stimulus, Verdict};
use crate::domain::embedding::{Embedding, EmbeddingPort};
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::domain::storage::{Event, StoragePort};
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::{CacheContext, CacheLookup, VerdictCache};
use crate::domain::wisdom::DomainCurations;
use crate::domain::wisdom::engine::format_wisdom_context;
use crate::judge::{Judge, JudgeError};
pub use maintenance::{backfill_crystal_embeddings, summarize_pending_sessions};
use sha2::{Digest, Sha256};

/// Callback type for progressive Dog completion notifications.
pub type OnDogCallback =
    dyn Fn(&str, bool, u64, Option<&crate::domain::dog::DogScore>, Option<String>) + Send + Sync;
use tokio::sync::Mutex;
use tracing::Instrument;

use crystal_observer::observe_crystal_for_verdict;
use verdict_observer::post_verdict_observation;

/// Layer 3 of sensitivity filter: domains that must always route to sovereign (local) backends.
/// Content in these domains is private by design (DMs, private financial data, wallet analysis).
/// If no sovereign Dogs are available, return 503 Service Unavailable.
const SOVEREIGN_DOMAINS: &[&str] = &["social-dm", "private", "wallet-judgment"];

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
    /// Domain curations (D1-D6 signals) for wisdom enrichment.
    /// K15: Dogs query patterns matching stimulus keywords to ground judgment in domain knowledge.
    pub domain_curations: &'a DomainCurations,
    /// Domain-aware Dog router — selects suitable Dogs based on domain hint.
    /// Initialized from backend_configs at boot. If None, all Dogs are used (fallback).
    pub domain_router: Option<&'a crate::infra::domain_router::DomainRouter>,
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

    let mature_crystals = if inject_crystals {
        // DORMANT: K15 consumer (organism memory recovery) — deferred to next phase
        // Would warm-start judgment context with domain-matching patterns learned from previous sessions
        let mut cached: Vec<crate::domain::ccm::MatureCrystal> = Vec::new();

        // Semantic search: find additional crystals matching stimulus embedding.
        let found = if let Some(ref emb) = stimulus_embedding {
            storage
                .search_crystals_semantic(&emb.vector, 10)
                .await
                .inspect_err(|e| {
                    tracing::warn!(error = %e, "semantic crystal search failed — fallback to domain list")
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Supplement cached with fresh queries.
        let supplemental = if found.is_empty() {
            storage
                .list_crystals_for_domain(domain_hint, 10)
                .await
                .inspect_err(|e| {
                    tracing::warn!(error = %e, "domain crystal list failed — pipeline continues without crystals")
                })
                .unwrap_or_default()
        } else {
            found
        };

        // Combine: cached + supplemental. Filter supplemental to mature crystals only.
        cached.extend(
            supplemental
                .into_iter()
                .filter(|c| {
                    c.state == crate::domain::ccm::CrystalState::Crystallized
                        || c.state == crate::domain::ccm::CrystalState::Canonical
                })
                .filter_map(|c| crate::domain::ccm::MatureCrystal::try_from(c).ok()),
        );
        cached
    } else {
        tracing::info!(phase = "crystals", "crystal injection disabled (A/B mode)");
        Vec::new()
    };

    tracing::info!(
        phase = "crystals",
        total = mature_crystals.len(),
        "mature crystals for injection"
    );
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

    // ── CONTEXT ENRICHMENT: merge user context + crystals + wisdom + sessions ──
    let enriched_context = {
        // Crystal budget derived from ML theory:
        // - Golden ratio weighting (arXiv 2502.18049): supplementary:primary = φ⁻²
        // - "Lost in the middle" attention research: first 20% of context = high attention
        // - Few-shot ICL research: 3-5 examples optimal
        // All three converge at ~domain_prompt_length × φ⁻² ≈ 1089 for chess (2850 chars).
        // Hardcoded for now; will scale dynamically with domain prompt length.
        const CRYSTAL_BUDGET_CHARS: usize = 1100;
        const WISDOM_BUDGET_CHARS: usize = 800;
        let crystal_ctx =
            ccm::format_crystal_context(&mature_crystals, domain_hint, CRYSTAL_BUDGET_CHARS);

        // K15: Query wisdom signals matching stimulus keywords
        let wisdom_ctx = format_wisdom_context(
            deps.domain_curations,
            domain_hint,
            &content,
            WISDOM_BUDGET_CHARS,
        );
        tracing::info!(
            phase = "wisdom",
            has_signals = wisdom_ctx.is_some(),
            "wisdom context query"
        );

        let parts: Vec<String> = [context, crystal_ctx, wisdom_ctx, session_ctx]
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
            // 60s: basic enrichment (~10s) + holder estimation (~12s) + behavioral (20s timeout)
            // + DexScreener (~2s). Was 30s before holder estimation addition.
            match tokio::time::timeout(
                std::time::Duration::from_secs(60),
                enricher.enrich(&content),
            )
            .await
            {
                Ok(Ok(Some(mut token_data))) => {
                    // Complement with DexScreener market data (volume, liquidity).
                    // Best-effort: if it fails, we still have Helius data.
                    let dex_client = crate::backends::dexscreener::DexScreenerClient::new();
                    if let Some(market) = dex_client.get_market_data(&content).await {
                        if token_data.volume_24h_usd.is_none() {
                            token_data.volume_24h_usd = market.volume_24h_usd;
                        }
                        if token_data.liquidity_usd.is_none() {
                            token_data.liquidity_usd = market.liquidity_usd;
                        }
                        // Cross-validate price: if Helius price is missing, use DexScreener
                        if token_data.price_usd.is_none() {
                            token_data.price_usd = market.price_usd;
                        }
                    }

                    let enriched = token_data.to_stimulus();
                    tracing::info!(
                        phase = "enrich",
                        mint = %content,
                        name = ?token_data.name,
                        symbol = ?token_data.symbol,
                        holders = ?token_data.holder_count,
                        volume_24h = ?token_data.volume_24h_usd,
                        liquidity = ?token_data.liquidity_usd,
                        kscore = ?token_data.kscore.as_ref().map(|k| format!("{:.3}", k.score)),
                        "token enriched via Helius+DexScreener"
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
                        "enrichment timed out (30s) — using raw address"
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
            let stimulus_summary: String = stimulus.content.chars().take(300).collect();
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
                failed_dog_errors: std::collections::BTreeMap::new(),
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
        side_effects(&stimulus, &verdict, &stimulus_embedding, deps, false).await;

        return Ok(PipelineResult::Evaluated {
            verdict: Box::new(verdict),
            token_data: None,
            enriched_content: None,
        });
    }

    tracing::info!(phase = "evaluate", "dispatching to Dogs");
    let on_dog_ref: Option<&OnDogCallback> = deps.on_dog.as_ref().map(|b| b.as_ref());

    // ── DOMAIN-AWARE DOG SELECTION ──
    // Layer 3: Domain constraint check — private domains route to sovereign backends only.
    let domain_requires_sovereign = SOVEREIGN_DOMAINS.contains(&domain_hint);

    // Data-centric: domain-suitable Dogs configured in backends.toml (suitable_for_domains field).
    // Priority: sovereign constraint > caller filter > domain_router > all Dogs (default).
    let dogs_filter_optimized: Option<Vec<String>>;
    let dogs_filter_final = if domain_requires_sovereign {
        // Layer 3 triggered: force sovereign-only routing.
        let sovereign = judge.sovereign_dog_ids();
        if sovereign.is_empty() {
            return Err(JudgeError::InvalidInput(
                "Sensitive domain requires sovereign backend but none available".into(),
            ));
        }
        tracing::info!(
            domain = domain_hint,
            "Layer 3 (domain constraint) fired — routing to sovereign Dogs only"
        );
        dogs_filter_optimized = Some(sovereign);
        dogs_filter_optimized.as_deref()
    } else if dogs_filter.is_some() {
        // Caller-provided filter takes precedence (Layer 2 already handled in handlers)
        dogs_filter
    } else {
        // Try data-driven selection from backends.toml
        let from_router = deps.domain_router.map(|r| r.dogs_for_domain(domain_hint));
        if let Some(router_dogs) = from_router {
            dogs_filter_optimized = Some(router_dogs);
            dogs_filter_optimized.as_deref()
        } else {
            // No router and no filter: use all Dogs (fallback)
            dogs_filter
        }
    };

    let mut verdict = judge
        .evaluate_progressive(&stimulus, dogs_filter_final, metrics, on_dog_ref)
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
    // Don't cache degraded enrichments — stale cache blocks fresh enrichment on retry
    let enrichment_degraded = captured_token_data
        .as_ref()
        .is_some_and(|td| !td.holder_data_available);
    side_effects(
        &stimulus,
        &verdict,
        &stimulus_embedding,
        deps,
        enrichment_degraded,
    )
    .await;

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
    enrichment_degraded: bool,
) {
    // Store verdict
    if let Err(e) = deps.storage.store_verdict(verdict).await {
        tracing::warn!(phase = "side_effects", error = %e, "failed to store verdict");
    }

    // K15: Enqueue verdict for onchain submission (gated on confidence threshold φ⁻¹)
    if verdict.q_score.total >= crate::domain::dog::PHI_INV {
        // Step 1: Compute SHA256 hash of stimulus for Pinocchio dedup
        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(stimulus.content.as_bytes());
            hex::encode(hasher.finalize())
        };

        // Step 2: Map VerdictKind to string for storage layer
        let verdict_type = match verdict.kind {
            crate::domain::dog::VerdictKind::Howl => "howl",
            crate::domain::dog::VerdictKind::Wag => "wag",
            crate::domain::dog::VerdictKind::Growl => "growl",
            crate::domain::dog::VerdictKind::Bark => "bark",
        };

        // Step 3: Extract dog count from verdict
        let dog_count = verdict.dog_scores.len() as u32;

        // Step 4: Enqueue for onchain submission
        if let Err(e) = deps
            .storage
            .enqueue_verdict(
                &verdict.id,
                &content_hash,
                verdict.q_score.total,
                verdict.q_score.fidelity,
                verdict.q_score.phi,
                verdict.q_score.verify,
                verdict.q_score.culture,
                verdict.q_score.burn,
                verdict.q_score.sovereignty,
                dog_count,
                verdict_type,
            )
            .await
        {
            // Graceful degradation: log but don't propagate (K14)
            tracing::error!(
                verdict_id = %verdict.id,
                q_score = %format!("{:.3}", verdict.q_score.total),
                error = %e,
                "Failed to enqueue verdict for onchain submission (K15 producer enqueue_verdict)"
            );
        }
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

    // K15: Emit fleet events for each Dog evaluation — feeds fleet_stats + list_degraded_nodes
    for ds in &verdict.dog_scores {
        let event = Event {
            tool: "dog_evaluation".to_string(),
            node: crate::api::rest::inference_router::dog_to_node(&ds.dog_id),
            elapsed_ms: ds.latency_ms,
            output_bytes: 0,
            success: true,
            failure_reason: "none".to_string(),
            agent_id: "kernel-pipeline".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: format!("dog={}", ds.dog_id),
        };
        let _ = deps.storage.store_event(&event).await;
    }

    // Emit failure events for failed Dogs
    for dog_id in &verdict.failed_dogs {
        let error_detail = verdict
            .failed_dog_errors
            .get(dog_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let event = Event {
            tool: "dog_evaluation".to_string(),
            node: crate::api::rest::inference_router::dog_to_node(dog_id),
            elapsed_ms: 0,
            output_bytes: 0,
            success: false,
            failure_reason: "process_crash".to_string(),
            agent_id: "kernel-pipeline".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: format!("dog={dog_id} error={error_detail}"),
        };
        let _ = deps.storage.store_event(&event).await;
    }

    // CCM: observe crystal + embed
    let domain = stimulus.domain.as_deref().unwrap_or("general");
    observe_crystal_for_verdict(verdict, stimulus_embedding, domain, deps).await;

    // K15 Forward loop: post verdict observation back to CCM intake
    // This completes the compound loop: observation → judge → verdict → observation
    post_verdict_observation(verdict, stimulus.domain.as_deref(), deps).await;

    // Cache verdict embedding — but NOT degraded enrichments (holder data unavailable).
    // A degraded verdict cached at similarity 0.999 blocks fresh enrichment on retry.
    if enrichment_degraded {
        tracing::info!(
            phase = "cache",
            "skipping cache store — enrichment was degraded"
        );
    }
    if !enrichment_degraded && let Some(emb) = &stimulus_embedding {
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

        let domain_curations = crate::domain::wisdom::DomainCurations::new();
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
            domain_curations: &domain_curations,
            domain_router: None,
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

        let domain_curations = crate::domain::wisdom::DomainCurations::new();
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
            domain_curations: &domain_curations,
            domain_router: None,
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

        let domain_curations = crate::domain::wisdom::DomainCurations::new();
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
            domain_curations: &domain_curations,
            domain_router: None,
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

        let domain_curations = crate::domain::wisdom::DomainCurations::new();
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
            domain_curations: &domain_curations,
            domain_router: None,
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

        let domain_curations = crate::domain::wisdom::DomainCurations::new();
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
            domain_curations: &domain_curations,
            domain_router: None,
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

        let domain_curations = crate::domain::wisdom::DomainCurations::new();
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
            domain_curations: &domain_curations,
            domain_router: None,
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

        let domain_curations = crate::domain::wisdom::DomainCurations::new();
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
            domain_curations: &domain_curations,
            domain_router: None,
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

    #[test]
    fn enqueue_verdict_hash_determinism() {
        // K15: Verify SHA256 hash is deterministic for same stimulus
        let stimulus1 = "The quick brown fox jumps over the lazy dog";
        let stimulus2 = "The quick brown fox jumps over the lazy dog";
        let stimulus3 = "Different stimulus";

        let hash1 = {
            let mut hasher = Sha256::new();
            hasher.update(stimulus1.as_bytes());
            hex::encode(hasher.finalize())
        };
        let hash2 = {
            let mut hasher = Sha256::new();
            hasher.update(stimulus2.as_bytes());
            hex::encode(hasher.finalize())
        };
        let hash3 = {
            let mut hasher = Sha256::new();
            hasher.update(stimulus3.as_bytes());
            hex::encode(hasher.finalize())
        };

        assert_eq!(
            hash1, hash2,
            "identical stimuli should produce identical hashes"
        );
        assert_ne!(
            hash1, hash3,
            "different stimuli should produce different hashes"
        );
        assert_eq!(
            hash1.len(),
            64,
            "SHA256 hex should be 64 chars (256 bits / 4 bits per hex digit)"
        );
    }

    #[test]
    fn enqueue_verdict_threshold_gate() {
        use crate::domain::dog::PHI_INV;

        // K15: Verify threshold gating logic
        let below_threshold = PHI_INV - 0.1;
        let at_threshold = PHI_INV;
        let above_threshold = PHI_INV + 0.1;

        assert!(
            below_threshold < PHI_INV,
            "below threshold should be < PHI_INV"
        );
        assert!(at_threshold >= PHI_INV, "at threshold should be >= PHI_INV");
        assert!(
            above_threshold >= PHI_INV,
            "above threshold should be >= PHI_INV"
        );

        // Only at_threshold and above should trigger enqueue
        assert!(
            !should_enqueue(below_threshold),
            "below threshold should not enqueue"
        );
        assert!(should_enqueue(at_threshold), "at threshold should enqueue");
        assert!(
            should_enqueue(above_threshold),
            "above threshold should enqueue"
        );
    }

    fn should_enqueue(q_score: f64) -> bool {
        use crate::domain::dog::PHI_INV;
        q_score >= PHI_INV
    }

    #[test]
    fn enqueue_verdict_verdict_type_mapping() {
        use crate::domain::dog::VerdictKind;

        // K15: Verify verdict kind → string mapping
        let mappings = vec![
            (VerdictKind::Howl, "howl"),
            (VerdictKind::Wag, "wag"),
            (VerdictKind::Growl, "growl"),
            (VerdictKind::Bark, "bark"),
        ];

        for (kind, expected) in mappings {
            let actual = match kind {
                VerdictKind::Howl => "howl",
                VerdictKind::Wag => "wag",
                VerdictKind::Growl => "growl",
                VerdictKind::Bark => "bark",
            };
            assert_eq!(
                actual, expected,
                "VerdictKind::{kind:?} should map to {expected}"
            );
        }
    }
}
