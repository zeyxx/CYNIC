use std::collections::HashMap;

use crate::domain::chunk::DocumentChunk;
use crate::domain::scorer::{
    SearchHit, evaluate_altitude_multiplier, evaluate_contract_floor,
    evaluate_deprecation_multiplier,
};

pub const DEFAULT_RRF_K: f64 = 60.0;

/// A dense vector retrieval hit holding a reference to a document chunk and cosine similarity.
#[derive(Debug, Clone)]
pub struct DenseHit<'a> {
    pub chunk: &'a DocumentChunk,
    pub similarity: f64,
}

/// Fuses sparse (BM25) and dense (vector embedding) rankings using Reciprocal Rank Fusion (RRF).
///
/// Formula:
///   RRF(d) = sum_{method} [ 1 / (k + rank_{method}(d)) ]
///   FinalScore(d) = [RRF(d) + ContractFloor] * Altitude(d) * Deprecation(d)
pub fn fuse_reciprocal_ranks<'a>(
    sparse_hits: &[SearchHit<'a>],
    dense_hits: &[DenseHit<'a>],
    k: f64,
) -> Vec<SearchHit<'a>> {
    let k = if k <= 0.0 { DEFAULT_RRF_K } else { k };

    let mut chunk_map: HashMap<&str, &'a DocumentChunk> = HashMap::new();
    let mut rrf_scores: HashMap<&str, f64> = HashMap::new();

    // 1. Process sparse (BM25) ranks (1-indexed)
    for (idx, hit) in sparse_hits.iter().enumerate() {
        let rank = (idx + 1) as f64;
        let path = hit.chunk.source_path.as_str();
        chunk_map.insert(path, hit.chunk);
        let score = 1.0 / (k + rank);
        *rrf_scores.entry(path).or_insert(0.0) += score;
    }

    // 2. Process dense (vector) ranks (1-indexed)
    for (idx, hit) in dense_hits.iter().enumerate() {
        let rank = (idx + 1) as f64;
        let path = hit.chunk.source_path.as_str();
        chunk_map.insert(path, hit.chunk);
        let score = 1.0 / (k + rank);
        *rrf_scores.entry(path).or_insert(0.0) += score;
    }

    // 3. Compute final weighted scores with CYNIC altitude and deprecation policy
    let mut fused = Vec::with_capacity(chunk_map.len());
    for (path, &chunk) in &chunk_map {
        let base_rrf = rrf_scores.get(path).copied().unwrap_or(0.0);
        let contract_floor = evaluate_contract_floor(path);
        let altitude = evaluate_altitude_multiplier(chunk.authority);
        let deprecation = evaluate_deprecation_multiplier(path, &chunk.content);

        // Normalize RRF by 100.0 so scale matches human-inspectable BM25 numbers
        let final_score = (base_rrf * 100.0 + contract_floor) * altitude * deprecation;

        fused.push(SearchHit {
            chunk,
            score: final_score,
        });
    }

    // 4. Sort in descending order of fused score
    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    fused
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::AuthorityTier;

    #[test]
    fn test_rrf_fuses_sparse_and_dense_rankings() {
        let chunk_a = DocumentChunk::new(
            "docs/foundation/CONSTITUTION.md".into(),
            1,
            10,
            vec!["Constitution".into()],
            "Power division invariant".into(),
            AuthorityTier::L0Constitution,
        );
        let chunk_b = DocumentChunk::new(
            "docs/specs/runtime.md".into(),
            1,
            10,
            vec!["Runtime".into()],
            "Execution pipeline specification".into(),
            AuthorityTier::L1Specification,
        );

        // Chunk A is #1 in sparse, #2 in dense
        let sparse = vec![
            SearchHit {
                chunk: &chunk_a,
                score: 10.0,
            },
            SearchHit {
                chunk: &chunk_b,
                score: 5.0,
            },
        ];

        // Chunk B is #1 in dense, #2 in sparse
        let dense = vec![
            DenseHit {
                chunk: &chunk_b,
                similarity: 0.95,
            },
            DenseHit {
                chunk: &chunk_a,
                similarity: 0.85,
            },
        ];

        let fused = fuse_reciprocal_ranks(&sparse, &dense, 60.0);
        assert_eq!(fused.len(), 2);

        // Chunk A should still lead because of L0 Constitution altitude boost (3.0 vs 2.0)
        assert_eq!(
            fused[0].chunk.source_path,
            "docs/foundation/CONSTITUTION.md"
        );
        assert_eq!(fused[0].chunk.authority, AuthorityTier::L0Constitution);
    }
}
