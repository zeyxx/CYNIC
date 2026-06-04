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

// ── CRYSTAL RELATION CURATION ─────────────────────────────

/// Discover and persist semantic relations between crystals.
/// This is the "Graph Curation" phase of GraphRAG.
/// It builds a structural memory model by linking similar patterns.
pub async fn curate_crystal_relations(storage: &dyn StoragePort, _metrics: &Metrics) -> u32 {
    let crystals = match storage.list_crystals(500).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "curate: failed to list crystals");
            return 0;
        }
    };

    let mature = ccm::filter_mature(crystals);
    if mature.is_empty() {
        return 0;
    }

    tracing::info!(
        phase = "curate",
        count = mature.len(),
        "starting crystal relation curation"
    );

    let mut updated_count = 0u32;
    for crystal in &mature {
        let Some(ref vector) = crystal.crystal().embedding else {
            continue;
        };

        // Find similar mature crystals
        let similar = match storage.search_crystals_semantic(vector, 10).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(crystal_id = %crystal.id(), error = %e, "curate: semantic search failed");
                continue;
            }
        };

        let mut relations = std::collections::BTreeMap::new();
        for s in similar {
            if s.id == crystal.id() {
                continue;
            }
            // Threshold for structural relation (high similarity)
            // We use 0.90 for SUPPORTS and 0.85 for RELATED
            // NOTE: search_crystals_semantic should return similarity in a field if possible,
            // but the Crystal struct doesn't have it.
            // Wait, search_crystals_semantic in surreal/crystals.rs uses 'AS similarity'.
            // But row_to_crystal doesn't populate it into the struct.
            // However, we can compute it manually here using Embedding::cosine_similarity
            // if we turn the vector into an Embedding.

            if let Some(ref s_vector) = s.embedding {
                let sim = cosine_similarity(vector, s_vector);
                if sim >= 0.95 {
                    relations.insert(s.id.clone(), "equivalent".to_string());
                } else if sim >= 0.85 {
                    relations.insert(s.id.clone(), "related".to_string());
                }
            }
        }

        if !relations.is_empty() {
            if let Err(e) = storage
                .update_crystal_relations(crystal.id(), relations)
                .await
            {
                tracing::warn!(crystal_id = %crystal.id(), error = %e, "curate: failed to update relations");
            } else {
                updated_count += 1;
            }
        }
    }

    tracing::info!(
        phase = "curate",
        updated = updated_count,
        "crystal relation curation complete"
    );
    updated_count
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b).map(|(x, y)| *x as f64 * *y as f64).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
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
