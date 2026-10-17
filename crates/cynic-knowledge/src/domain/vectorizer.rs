//! Pure chunk vectorizer: batch-embed corpus chunks, rank by cosine.
//!
//! Sync port on purpose — `cynic-knowledge` stays free of async runtimes.
//! The live HTTP backend adapts at the wiring layer, outside this crate.
//! Vectors join chunks by the BLAKE3 `chunk_id`, never by position.

use std::collections::HashMap;

use crate::domain::chunk::DocumentChunk;
use crate::domain::hybrid_rrf::DenseHit;

/// Minimal embedding backend: one batch call, dense vectors out.
pub trait ChunkEmbedder {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String>;

    /// Batch with per-call progress: `report(done, total)` fires as workers
    /// complete sublots. Cluster backends override this so ONE shared queue
    /// spans the whole slice; never chunk before fanout or the endpoints
    /// after the first are starved by construction. The closure must be
    /// `Sync` because receiving workers call it concurrently.
    fn embed_batch_reported(
        &self,
        texts: &[&str],
        report: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<Vec<Vec<f32>>, String> {
        let out = self.embed_batch(texts)?;
        report(out.len(), texts.len());
        Ok(out)
    }

    /// Chunk-aware entry: defaults to `embed_batch` over contents, but lets
    /// adapters format per-chunk input texts (titles, instruction prefixes)
    /// from chunk metadata without changing the trait for existing backends.
    fn embed_chunks(&self, chunks: &[DocumentChunk]) -> Result<Vec<Vec<f32>>, String> {
        let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
        self.embed_batch(&texts)
    }
}

/// A chunk keyed by its stable BLAKE3 id, paired with its dense vector.
#[derive(Debug, Clone)]
pub struct ChunkVector {
    pub chunk_id: String,
    pub vector: Vec<f32>,
}

/// Embed every chunk's content through `embedder`, preserving chunk order.
/// Fails closed when the backend returns a vector per chunk mismatch.
pub fn vectorize_chunks(
    embedder: &dyn ChunkEmbedder,
    chunks: &[DocumentChunk],
) -> Result<Vec<ChunkVector>, String> {
    let vectors = embedder.embed_chunks(chunks)?;
    if vectors.len() != chunks.len() {
        return Err(format!(
            "embedder returned {} vectors for {} chunks",
            vectors.len(),
            chunks.len()
        ));
    }
    Ok(chunks
        .iter()
        .zip(vectors)
        .map(|(chunk, vector)| ChunkVector {
            chunk_id: chunk.chunk_id.clone(),
            vector,
        })
        .collect())
}

/// Cosine similarity. Zero on dimension mismatch or null norm — never panics,
/// so a corrupt vector degrades a hit instead of killing the search.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();
    let norm_a: f64 = a.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Dense top-k over `chunks`, joined by `chunk_id`. Sorted by descending
/// similarity, ties broken by `chunk_id` for determinism. Feeds
/// `fuse_reciprocal_ranks` directly.
pub fn top_k_dense<'a>(
    query: &[f32],
    chunks: &'a [DocumentChunk],
    vectors: &[ChunkVector],
    k: usize,
) -> Vec<DenseHit<'a>> {
    let by_id: HashMap<&str, &[f32]> = vectors
        .iter()
        .map(|v| (v.chunk_id.as_str(), v.vector.as_slice()))
        .collect();
    let mut scored: Vec<DenseHit<'a>> = chunks
        .iter()
        .filter_map(|chunk| {
            by_id.get(chunk.chunk_id.as_str()).map(|vector| DenseHit {
                chunk,
                similarity: cosine_similarity(query, vector),
            })
        })
        .collect();
    scored.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
    });
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::AuthorityTier;

    struct StubEmbedder;
    impl ChunkEmbedder for StubEmbedder {
        fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts
                .iter()
                .map(|text| {
                    let bytes = text.as_bytes();
                    vec![
                        f32::from(bytes.first().copied().unwrap_or(0)),
                        (bytes.len() % 7) as f32,
                        1.0,
                    ]
                })
                .collect())
        }
    }

    struct ShortEmbedder;
    impl ChunkEmbedder for ShortEmbedder {
        fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts.iter().take(1).map(|_| vec![1.0]).collect())
        }
    }

    fn chunk(path: &str, content: &str) -> DocumentChunk {
        DocumentChunk::new(
            path.into(),
            1,
            3,
            vec![],
            content.into(),
            AuthorityTier::L3Operation,
        )
    }

    #[test]
    fn vectorize_preserves_chunk_order_and_ids() {
        let chunks = vec![chunk("a.md", "alpha"), chunk("b.md", "beta")];
        let vectors = vectorize_chunks(&StubEmbedder, &chunks).expect("vectors");
        assert_eq!(vectors.len(), 2);
        assert_eq!(vectors[0].chunk_id, chunks[0].chunk_id);
        assert_eq!(vectors[1].chunk_id, chunks[1].chunk_id);
    }

    #[test]
    fn vectorize_fails_closed_on_count_mismatch() {
        let chunks = vec![chunk("a.md", "alpha"), chunk("b.md", "beta")];
        assert!(vectorize_chunks(&ShortEmbedder, &chunks).is_err());
    }

    #[test]
    fn cosine_is_one_for_identical_one_for_scaled() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!((cosine_similarity(&[1.0, 2.0], &[2.0, 4.0]) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
    }

    #[test]
    fn cosine_degrades_to_zero_without_panicking() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0]), 0.0);
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }

    #[test]
    fn top_k_ranks_and_caps_deterministically() {
        let chunks = vec![
            chunk("a.md", "alpha alpha"),
            chunk("b.md", "beta"),
            chunk("c.md", "alpha beta gamma delta"),
        ];
        let vectors = vectorize_chunks(&StubEmbedder, &chunks).expect("vectors");
        let query = vec![97.0, 3.0, 1.0];
        let top1 = top_k_dense(&query, &chunks, &vectors, 1);
        assert_eq!(top1.len(), 1);
        let top3 = top_k_dense(&query, &chunks, &vectors, 3);
        assert_eq!(top3.len(), 3);
        assert!(top3[0].similarity >= top3[1].similarity);
        assert!(top3[1].similarity >= top3[2].similarity);
        // Deterministic rerun: identical order.
        let rerun = top_k_dense(&query, &chunks, &vectors, 3);
        let ids: Vec<&str> = top3.iter().map(|h| h.chunk.chunk_id.as_str()).collect();
        let rerun_ids: Vec<&str> = rerun.iter().map(|h| h.chunk.chunk_id.as_str()).collect();
        assert_eq!(ids, rerun_ids);
    }

    #[test]
    fn top_k_skips_chunks_without_vectors() {
        let chunks = vec![chunk("a.md", "alpha"), chunk("ghost.md", "ghost")];
        let mut vectors = vectorize_chunks(&StubEmbedder, &chunks).expect("vectors");
        vectors.pop();
        let hits = top_k_dense(&[97.0, 1.0, 1.0], &chunks, &vectors, 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].chunk.source_path, "a.md");
    }
}
