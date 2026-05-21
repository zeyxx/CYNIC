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

            // Look up trajectory from daily cron observations.
            // The cron POSTs one observation with all tracked tokens' decay curves.
            // Query recent observations and find the trajectory_cron entry for this mint.
            if let Ok(obs_list) = deps
                .storage
                .list_observations_raw(Some("token-analysis"), None, 20)
                .await
            {
                for obs in &obs_list {
                    if obs.tool != "trajectory_cron" {
                        continue;
                    }
                    if let Ok(ctx) = serde_json::from_str::<serde_json::Value>(&obs.context)
                        && let Some(trajectories) =
                            ctx.get("trajectories").and_then(|t| t.as_array())
                    {
                        for traj in trajectories {
                            if traj.get("mint").and_then(|m| m.as_str()) == Some(&content) {
                                token_data.trajectory_class = traj
                                    .get("class")
                                    .and_then(|c| c.as_str())
                                    .map(|s| s.to_string());
                                token_data.trajectory_decay =
                                    traj.get("decay").and_then(|d| d.as_f64());
                                break;
                            }
                        }
                    }
                    break; // only need the latest trajectory observation
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
                trajectory = ?token_data.trajectory_class,
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

/// Phone number enrichment — aggregates observations into a structured stimulus.
///
/// When domain=phone-number and content looks like a phone number,
/// queries storage for existing observations, aggregates them into PhoneData,
/// and replaces the raw number with a structured stimulus that Dogs can evaluate.
pub(super) async fn enrich_phone(
    content: &mut String,
    domain_hint: &str,
    storage: &dyn crate::domain::storage::StoragePort,
) {
    if domain_hint != "phone-number" {
        return;
    }

    let number = content.trim().to_string();
    if number.is_empty() {
        return;
    }

    // Query observations for this phone number
    let Ok(observations) = storage
        .list_observations_raw(Some("phone-number"), None, 500)
        .await
    else {
        return; // Storage down — use raw content (presumption of innocence)
    };

    // Filter observations matching this number
    let matching: Vec<_> = observations.iter().filter(|o| o.target == number).collect();

    if matching.is_empty() {
        // No observations — build minimal PhoneData from the raw number
        let country = if number.starts_with("+33") {
            "FR"
        } else if number.starts_with("+1") {
            "US"
        } else {
            "XX"
        };
        let data = crate::domain::phone_number::PhoneData {
            number: number.clone(),
            country_code: country.to_string(),
            total_events: 0,
            label_distribution: crate::domain::phone_number::LabelDistribution::default(),
            reporter_count: 0,
            mean_reporter_trust: 0.0,
            age_days: 0,
            days_since_last_report: 0,
            challenge_pass_rate: None,
            contestation_count: 0,
            owner_verified: false,
        };
        *content = crate::domain::stimulus::build_phone_stimulus(&data);
        return;
    }

    // Aggregate observations into PhoneData
    let mut legit = 0u32;
    let mut nuisance = 0u32;
    let mut scam = 0u32;
    let mut unknown = 0u32;
    let mut reporter_count = 0u32;
    let mut trust_sum = 0.0f32;
    let mut trust_count = 0u32;

    for obs in &matching {
        // Parse context JSON for label + metadata
        if let Ok(ctx) = serde_json::from_str::<serde_json::Value>(&obs.context) {
            let label = ctx
                .get("ground_truth")
                .or_else(|| ctx.get("label"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            match label {
                "legitimate" | "legit" => legit += 1,
                "nuisance" => nuisance += 1,
                "scam" | "spam" => scam += 1,
                _ => unknown += 1,
            }

            if let Some(rc) = ctx.get("reporter_count").and_then(|v| v.as_u64()) {
                reporter_count = reporter_count.max(rc as u32);
            } else {
                reporter_count += 1;
            }

            if let Some(trust) = ctx.get("mean_reporter_trust").and_then(|v| v.as_f64()) {
                trust_sum += trust as f32;
                trust_count += 1;
            }
        } else {
            unknown += 1;
            reporter_count += 1;
        }
    }

    let country = if number.starts_with("+33") {
        "FR"
    } else if number.starts_with("+1") {
        "US"
    } else {
        "XX"
    };

    let data = crate::domain::phone_number::PhoneData {
        number: number.clone(),
        country_code: country.to_string(),
        total_events: matching.len() as u64,
        label_distribution: crate::domain::phone_number::LabelDistribution {
            legitimate: legit,
            nuisance,
            scam,
            unknown,
        },
        reporter_count,
        mean_reporter_trust: if trust_count > 0 {
            trust_sum / trust_count as f32
        } else {
            0.5 // default moderate trust
        },
        age_days: 0, // TODO: compute from observation timestamps
        days_since_last_report: 0,
        challenge_pass_rate: None,
        contestation_count: 0,
        owner_verified: false,
    };

    *content = crate::domain::stimulus::build_phone_stimulus(&data);
}
