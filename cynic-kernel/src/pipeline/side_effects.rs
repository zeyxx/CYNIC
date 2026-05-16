//! Best-effort side effects after verdict evaluation.
//!
//! Store verdict, enqueue for onchain, track usage, emit fleet events,
//! observe crystals, post verdict observation, cache embedding.

use crate::domain::dog::Verdict;
use crate::domain::dog::{PHI_INV, Stimulus};
use crate::domain::embedding::Embedding;
use crate::domain::storage::Event;
use crate::domain::verdict_cache::CacheContext;
use sha2::{Digest, Sha256};

use super::PipelineDeps;
use super::crystal_observer::observe_crystal_for_verdict;
use super::verdict_observer::post_verdict_observation;

/// Run all best-effort side effects after verdict evaluation.
/// Never fails — all errors are logged and swallowed.
pub(super) async fn run(
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
    if verdict.q_score.total >= PHI_INV {
        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(stimulus.content.as_bytes());
            hex::encode(hasher.finalize())
        };

        let verdict_type = match verdict.kind {
            crate::domain::dog::VerdictKind::Howl => "howl",
            crate::domain::dog::VerdictKind::Wag => "wag",
            crate::domain::dog::VerdictKind::Growl => "growl",
            crate::domain::dog::VerdictKind::Bark => "bark",
        };

        let dog_count = verdict.dog_scores.len() as u32;

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

    // K15 Forward loop: verdict → observation → CCM → crystals → Dog prompts.
    // Nightshift picks these up with Background priority (Soma L2).
    post_verdict_observation(verdict, stimulus.domain.as_deref(), deps).await;

    // Cache verdict embedding — but NOT degraded enrichments (holder data unavailable).
    if enrichment_degraded {
        tracing::info!(
            phase = "cache",
            "skipping cache store — enrichment was degraded"
        );
    }
    if !enrichment_degraded && let Some(emb) = stimulus_embedding {
        let cache_emb = Embedding {
            vector: emb.vector.clone(),
            dimensions: emb.dimensions,
            prompt_tokens: emb.prompt_tokens,
        };
        let dog_id_hash = {
            let mut h: u64 = 0xcbf29ce484222325;
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
