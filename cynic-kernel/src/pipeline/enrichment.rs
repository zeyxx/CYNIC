//! Pipeline enrichment stages — context preparation before Dog evaluation.
//!
//! Stages: embed → cache check → crystal loading → context merge → token enrichment → wallet enrichment.

use crate::domain::ccm;
use crate::domain::embedding::Embedding;
use crate::domain::verdict_cache::{CacheContext, CacheLookup};
use crate::domain::wisdom::engine::format_wisdom_context;
use crate::judge::Judge;

use super::crystal_observer::observe_crystal_for_verdict;
use super::{PipelineDeps, PipelineResult};

/// Embed the stimulus content for cache lookup and semantic crystal retrieval.
pub(super) async fn embed_stimulus(content: &str, deps: &PipelineDeps<'_>) -> Option<Embedding> {
    match deps.embedding.embed(content).await {
        Ok(emb) => {
            deps.metrics.inc_embed_ok();
            tracing::info!(phase = "embed", dimensions = emb.dimensions, "embedding ok");
            Some(emb)
        }
        Err(e) => {
            deps.metrics.inc_embed_fail();
            tracing::warn!(phase = "embed", error = %e, "embedding failed — cache + crystal merge disabled");
            None
        }
    }
}

/// Check the verdict cache for a near-identical stimulus.
/// Returns `Some(PipelineResult::CacheHit)` on hit, `None` on miss.
pub(super) async fn check_cache(
    stimulus_embedding: &Option<Embedding>,
    domain_hint: &str,
    inject_crystals: bool,
    judge: &Judge,
    dogs_filter: Option<&[String]>,
    deps: &PipelineDeps<'_>,
) -> Option<PipelineResult> {
    let cache_ctx = CacheContext::new(domain_hint, judge.available_dogs_hash(dogs_filter));
    if inject_crystals
        && let Some(emb) = stimulus_embedding
        && let CacheLookup::Hit {
            verdict,
            similarity,
        } = deps.verdict_cache.lookup(emb, &cache_ctx)
    {
        deps.metrics.inc_cache_hit();
        tracing::info!(phase = "cache", result = "hit", similarity = %format!("{:.4}", similarity),
            verdict_id = %verdict.id, q_score = %format!("{:.3}", verdict.q_score.total));
        observe_crystal_for_verdict(&verdict, stimulus_embedding, domain_hint, deps).await;
        return Some(PipelineResult::CacheHit {
            verdict,
            similarity,
        });
    }
    deps.metrics.inc_cache_miss();
    tracing::info!(phase = "cache", result = "miss");
    None
}

/// Load mature crystals for context injection.
pub(super) async fn load_crystals(
    stimulus_embedding: &Option<Embedding>,
    domain_hint: &str,
    inject_crystals: bool,
    deps: &PipelineDeps<'_>,
) -> Vec<ccm::MatureCrystal> {
    if !inject_crystals {
        tracing::info!(phase = "crystals", "crystal injection disabled (A/B mode)");
        return Vec::new();
    }

    let mut cached: Vec<ccm::MatureCrystal> = Vec::new();

    let found = if let Some(emb) = stimulus_embedding {
        deps.storage
            .search_crystals_semantic(&emb.vector, 10)
            .await
            .inspect_err(|e| {
                tracing::warn!(error = %e, "semantic crystal search failed — fallback to domain list")
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let supplemental = if found.is_empty() {
        deps.storage
            .list_crystals_for_domain(domain_hint, 10)
            .await
            .inspect_err(|e| {
                tracing::warn!(error = %e, "domain crystal list failed — pipeline continues without crystals")
            })
            .unwrap_or_default()
    } else {
        found
    };

    cached.extend(
        supplemental
            .into_iter()
            .filter(|c| {
                c.state == ccm::CrystalState::Crystallized
                    || c.state == ccm::CrystalState::Canonical
            })
            .filter_map(|c| ccm::MatureCrystal::try_from(c).ok()),
    );

    tracing::info!(
        phase = "crystals",
        total = cached.len(),
        "mature crystals for injection"
    );
    cached
}

/// Merge user context + crystals + wisdom + session summaries into enriched context.
pub(super) async fn merge_context(
    context: Option<String>,
    mature_crystals: &[ccm::MatureCrystal],
    domain_hint: &str,
    content: &str,
    deps: &PipelineDeps<'_>,
) -> Option<String> {
    const CRYSTAL_BUDGET_CHARS: usize = 1100;
    const WISDOM_BUDGET_CHARS: usize = 800;

    let crystal_ctx =
        ccm::format_crystal_context(mature_crystals, domain_hint, CRYSTAL_BUDGET_CHARS);

    let wisdom_ctx = format_wisdom_context(
        deps.domain_curations,
        domain_hint,
        content,
        WISDOM_BUDGET_CHARS,
    );
    tracing::info!(
        phase = "wisdom",
        has_signals = wisdom_ctx.is_some(),
        "wisdom context query"
    );

    let session_ctx = deps.storage.list_session_summaries(5).await
        .inspect_err(|e| tracing::warn!(error = %e, "session summaries failed — pipeline continues without session context"))
        .ok()
        .and_then(|s| ccm::format_session_context(&s, 400));

    let parts: Vec<String> = [context, crystal_ctx, wisdom_ctx, session_ctx]
        .into_iter()
        .flatten()
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Output of token enrichment — captures both the rewritten stimulus and the raw data.
pub(super) struct TokenEnrichmentResult {
    pub content: String,
    pub token_data: Option<crate::domain::enrichment::TokenData>,
}

/// Enrich stimulus with on-chain token data when domain=token-analysis.
pub(super) async fn enrich_token(
    content: String,
    domain_hint: &str,
    deps: &PipelineDeps<'_>,
) -> TokenEnrichmentResult {
    if domain_hint != "token-analysis"
        || !crate::domain::enrichment::looks_like_solana_address(&content)
    {
        return TokenEnrichmentResult {
            content,
            token_data: None,
        };
    }

    let Some(enricher) = deps.enricher else {
        tracing::debug!(
            phase = "enrich",
            "no enricher configured — using raw address"
        );
        return TokenEnrichmentResult {
            content,
            token_data: None,
        };
    };

    match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        enricher.enrich(&content),
    )
    .await
    {
        Ok(Ok(Some(mut token_data))) => {
            let dex_client = crate::backends::dexscreener::DexScreenerClient::new();
            if let Some(market) = dex_client.get_market_data(&content).await {
                if token_data.volume_24h_usd.is_none() {
                    token_data.volume_24h_usd = market.volume_24h_usd;
                }
                if token_data.liquidity_usd.is_none() {
                    token_data.liquidity_usd = market.liquidity_usd;
                }
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
            TokenEnrichmentResult {
                content: enriched,
                token_data: Some(token_data),
            }
        }
        Ok(Ok(None)) => {
            tracing::warn!(phase = "enrich", mint = %content, "not a recognized token — using raw address");
            TokenEnrichmentResult {
                content,
                token_data: None,
            }
        }
        Ok(Err(e)) => {
            tracing::warn!(phase = "enrich", error = %e, "enrichment failed — using raw address");
            TokenEnrichmentResult {
                content,
                token_data: None,
            }
        }
        Err(_) => {
            tracing::warn!(
                phase = "enrich",
                "enrichment timed out (60s) — using raw address"
            );
            TokenEnrichmentResult {
                content,
                token_data: None,
            }
        }
    }
}

/// Enrich stimulus with structured wallet metrics when domain=wallet-judgment.
pub(super) fn enrich_wallet(
    content: String,
    domain_hint: &str,
    wallet_context_json: &Option<String>,
) -> (
    String,
    Option<crate::domain::wallet_judgment::WalletProfile>,
) {
    if domain_hint != "wallet-judgment" {
        return (content, None);
    }

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
            (stimulus, Some(profile))
        }
        None => {
            tracing::warn!(
                phase = "enrich",
                "wallet-judgment: no valid WalletProfile JSON in context — using raw content"
            );
            (content, None)
        }
    }
}
