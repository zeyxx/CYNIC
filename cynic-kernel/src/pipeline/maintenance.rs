//! Pipeline maintenance — background tasks that run independently of the judge flow.
//! Split from mod.rs per K16: the hot eval path should not carry maintenance code.

use crate::domain::ccm;
use crate::domain::embedding::EmbeddingPort;
use crate::domain::metrics::Metrics;
use crate::domain::storage::StoragePort;

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
    use crate::domain::storage::NullStorage;
    use crate::domain::summarization::NullSummarizer;

    #[tokio::test]
    async fn summarize_pending_sessions_with_null_deps_returns_zero() {
        let count = summarize_pending_sessions(&NullStorage, &NullSummarizer).await;
        assert_eq!(count, 0, "NullStorage has no pending sessions");
    }
}
