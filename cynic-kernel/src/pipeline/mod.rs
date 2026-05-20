//! Judge Pipeline — shared evaluation logic for REST and MCP.
//!
//! This is an application service: it orchestrates domain logic (Judge, CCM)
//! with port calls (StoragePort, EmbeddingPort, VerdictCache).
//! Handlers call this, then format the response for their transport.
//!
//! Stages: embed → cache → crystals → context → enrich → evaluate → side-effects.
//! Each stage lives in its own submodule for parallel development.

mod crystal_observer;
mod enrichment;
mod evaluation;
pub mod maintenance;
pub(crate) mod side_effects;
mod verdict_observer;

pub use crystal_observer::observe_crystal_for_verdict_core;
pub use maintenance::{backfill_crystal_embeddings, summarize_pending_sessions};

use crate::domain::dog::Stimulus;
use crate::domain::embedding::EmbeddingPort;
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::domain::slot_semaphore::SlotPriority;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::VerdictCache;
use crate::domain::wisdom::DomainCurations;
use crate::judge::{Judge, JudgeError};
use tokio::sync::Mutex;
use tracing::Instrument;

/// Callback type for progressive Dog completion notifications.
pub type OnDogCallback =
    dyn Fn(&str, bool, u64, Option<&crate::domain::dog::DogScore>, Option<String>) + Send + Sync;

/// Layer 3 of sensitivity filter: domains that must always route to sovereign (local) backends.
/// Content in these domains is private by design (DMs, private financial data, wallet analysis).
/// If no sovereign Dogs are available, return 503 Service Unavailable.
const SOVEREIGN_DOMAINS: &[&str] = &["social-dm", "private", "wallet-judgment", "phone-number"];

/// Result of the judge pipeline — everything a handler needs to build its response.
#[derive(Debug)]
pub enum PipelineResult {
    /// Cache hit — near-identical stimulus already judged.
    CacheHit {
        verdict: Box<crate::domain::dog::Verdict>,
        similarity: f64,
    },
    /// Full evaluation — fresh verdict.
    Evaluated {
        verdict: Box<crate::domain::dog::Verdict>,
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
    /// Priority tier for slot acquisition — determines wait behaviour per caller.
    /// REST/MCP handlers pass User; nightshift passes Nightshift; background passes Background.
    pub priority: SlotPriority,
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

    pipeline_inner(content, context, domain, dogs_filter, inject_crystals, deps)
        .instrument(pipeline_span)
        .await
}

/// Core pipeline orchestrator — calls stage functions in sequence.
async fn pipeline_inner(
    content: String,
    context: Option<String>,
    domain: Option<String>,
    dogs_filter: Option<&[String]>,
    inject_crystals: bool,
    deps: &PipelineDeps<'_>,
) -> Result<PipelineResult, JudgeError> {
    tracing::info!(content_len = content.len(), "pipeline_inner() started");
    let domain_hint = domain.as_deref().unwrap_or("general");

    // ── Stage 1: Embed ──
    let stimulus_embedding = enrichment::embed_stimulus(&content, deps).await;

    // ── Stage 2: Cache check ──
    // Skip cache for domains with dynamic enrichment (token-analysis, wallet-judgment).
    // These domains fetch live on-chain data per-request — caching on the raw content
    // (mint address) returns stale verdicts when data changes or bugs are fixed.
    let has_dynamic_enrichment = domain_hint == "token-analysis"
        || domain_hint == "wallet-judgment"
        || domain_hint == "phone-number";
    if !has_dynamic_enrichment
        && let Some(hit) = enrichment::check_cache(
            &stimulus_embedding,
            domain_hint,
            inject_crystals,
            deps.judge,
            dogs_filter,
            deps,
        )
        .await
    {
        return Ok(hit);
    }

    // ── Stage 3: Crystal loading ──
    let mature_crystals =
        enrichment::load_crystals(&stimulus_embedding, domain_hint, inject_crystals, deps).await;

    // ── Stage 4: Context merge ──
    let wallet_context_json = if domain_hint == "wallet-judgment" {
        context.clone()
    } else {
        None
    };
    let enriched_context =
        enrichment::merge_context(context, &mature_crystals, domain_hint, &content, deps).await;

    // ── Stage 5: Token enrichment ──
    let token_result = enrichment::enrich_token(content, domain_hint, deps).await;
    let content = token_result.content;
    let captured_token_data = token_result.token_data;
    let enriched_content = if captured_token_data.is_some() {
        Some(content.clone())
    } else {
        None
    };

    // ── Stage 6: Wallet enrichment ──
    let (mut content, captured_wallet_profile) =
        enrichment::enrich_wallet(content, domain_hint, &wallet_context_json);

    // ── Stage 6b: Phone-number enrichment ──
    enrichment::enrich_phone(&mut content, domain_hint);

    // ── Stage 7: Build stimulus ──
    let stimulus = Stimulus {
        content,
        context: enriched_context,
        domain: domain.clone(),
        request_id: deps.request_id.clone(),
    };

    // ── Stage 8: Wallet fast-path (deterministic, no LLM Dogs) ──
    if let Some(result) = evaluation::wallet_judgment_fast_path(
        &stimulus,
        captured_wallet_profile.as_ref(),
        domain_hint,
        &stimulus_embedding,
        deps,
    )
    .await?
    {
        return Ok(result);
    }

    // ── Stage 9: Dog selection + evaluation ──
    tracing::info!(phase = "evaluate", "dispatching to Dogs");
    let on_dog_ref: Option<&OnDogCallback> = deps.on_dog.as_ref().map(|b| b.as_ref());

    let (owned_filter, borrowed_filter) =
        evaluation::select_dogs(domain_hint, dogs_filter, deps.judge, deps)?;
    let dogs_filter_final = owned_filter.as_deref().or(borrowed_filter);

    let mut verdict = deps
        .judge
        .evaluate_progressive(
            &stimulus,
            dogs_filter_final,
            deps.metrics,
            on_dog_ref,
            deps.priority,
        )
        .await?;
    deps.metrics.inc_verdict();
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

    // ── Stage 10: K14 jury gate ──
    evaluation::jury_gate(&mut verdict, dogs_filter, deps);

    // ── Stage 11: Event emission ──
    let domain_str = stimulus.domain.as_deref().unwrap_or("general");
    evaluation::emit_verdict_event(&verdict, domain_str, deps);

    // ── Stage 12: Side effects ──
    let enrichment_degraded = captured_token_data
        .as_ref()
        .is_some_and(|td| !td.holder_data_available);
    side_effects::run(
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

#[cfg(test)]
mod tests;
