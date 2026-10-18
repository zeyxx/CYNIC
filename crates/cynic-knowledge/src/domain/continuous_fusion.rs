//! Continuous Score Fusion & Calibration (Zero-I/O Domain Module)
//!
//! Replaces ordinal Reciprocal Rank Fusion (RRF) with continuous convex combination:
//!
//! $$S(d) = \left( \alpha \cdot \hat{S}_{\text{dense}}(d) + (1 - \alpha) \cdot \hat{S}_{\text{bm25}}(d) + \text{floor} \right) \cdot \text{altitude} \cdot \text{deprecation}$$
//!
//! Where $\hat{S}$ maps raw heterogeneous score distributions (heavy-tailed BM25 and bounded cosine)
//! into commensurable $[0.0, 1.0]$ confidence signals.

use std::collections::HashMap;

use crate::domain::chunk::DocumentChunk;
use crate::domain::hybrid_rrf::DenseHit;
use crate::domain::scorer::{
    SearchHit, evaluate_altitude_multiplier, evaluate_contract_floor,
    evaluate_deprecation_multiplier,
};

/// Strategy used to project raw score distributions into commensurable $[0.0, 1.0]$ intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationStrategy {
    /// Relative min-max scaling over the candidate set:
    /// $\hat{s} = (s - s_{\min}) / (s_{\max} - s_{\min})$.
    CandidateMinMax,
    /// Absolute clamping:
    /// $\hat{s} = \text{clamp01}((s - \text{floor}) / (\text{ceil} - \text{floor}))$.
    AbsoluteBand,
    /// Empirical Cumulative Distribution Function percentile ranking:
    /// $\hat{s} = \text{rank}(s) / N$.
    EmpiricalCdf,
}

/// Computes the empirical cumulative distribution percentile for a value in a sorted sample.
#[derive(Debug, Clone)]
pub struct EmpiricalCdf {
    /// Ascending sorted score samples.
    sorted_samples: Vec<f64>,
}

impl EmpiricalCdf {
    /// Constructs an ECDF from a collection of observed raw scores.
    pub fn from_samples(mut samples: Vec<f64>) -> Self {
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self {
            sorted_samples: samples,
        }
    }

    /// Evaluates the percentile rank of `val` in $[0.0, 1.0]$.
    pub fn percentile(&self, val: f64) -> f64 {
        if self.sorted_samples.is_empty() {
            return 0.0;
        }
        let count_below = self
            .sorted_samples
            .iter()
            .take_while(|&&sample| sample <= val)
            .count();
        count_below as f64 / self.sorted_samples.len() as f64
    }
}

/// Normalizes a slice of candidate scores into $[0.0, 1.0]$ using candidate Min-Max scaling.
pub fn min_max_normalize(scores: &[f64]) -> Vec<f64> {
    if scores.is_empty() {
        return Vec::new();
    }
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &s in scores {
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
    }
    if (max - min).abs() < f64::EPSILON {
        return vec![1.0; scores.len()];
    }
    scores
        .iter()
        .map(|&s| ((s - min) / (max - min)).clamp(0.0, 1.0))
        .collect()
}

/// Fuses sparse (BM25) and dense (vector) hits using a continuous convex combination:
///
/// $$S(d) = \left( \alpha \cdot \hat{S}_{\text{dense}} + (1 - \alpha) \cdot \hat{S}_{\text{bm25}} + \text{floor} \right) \cdot \text{altitude} \cdot \text{deprecation}$$
///
/// Both rankings are keyed on pure `chunk.chunk_id` to guarantee fragment identity.
pub fn fuse_continuous_convex<'a>(
    sparse_hits: &[SearchHit<'a>],
    dense_hits: &[DenseHit<'a>],
    alpha: f64,
    strategy: NormalizationStrategy,
) -> Vec<SearchHit<'a>> {
    let alpha = alpha.clamp(0.0, 1.0);
    let mut chunk_map: HashMap<&str, &'a DocumentChunk> = HashMap::new();
    let mut sparse_scores: HashMap<&str, f64> = HashMap::new();
    let mut dense_scores: HashMap<&str, f64> = HashMap::new();

    // 1. Collect sparse raw scores
    let raw_sparse_values: Vec<f64> = sparse_hits.iter().map(|h| h.score).collect();
    let norm_sparse = match strategy {
        NormalizationStrategy::CandidateMinMax => min_max_normalize(&raw_sparse_values),
        NormalizationStrategy::AbsoluteBand => {
            // BM25 scores typically range [0.0 .. 30.0]
            raw_sparse_values
                .iter()
                .map(|&s| (s / 30.0).clamp(0.0, 1.0))
                .collect()
        }
        NormalizationStrategy::EmpiricalCdf => {
            let ecdf = EmpiricalCdf::from_samples(raw_sparse_values.clone());
            raw_sparse_values
                .iter()
                .map(|&s| ecdf.percentile(s))
                .collect()
        }
    };

    for (hit, &norm_score) in sparse_hits.iter().zip(norm_sparse.iter()) {
        let key = hit.chunk.chunk_id.as_str();
        chunk_map.insert(key, hit.chunk);
        sparse_scores.insert(key, norm_score);
    }

    // 2. Collect dense raw scores (cosine similarities in [-1.0, 1.0])
    let raw_dense_values: Vec<f64> = dense_hits.iter().map(|h| h.similarity as f64).collect();
    let norm_dense = match strategy {
        NormalizationStrategy::CandidateMinMax => min_max_normalize(&raw_dense_values),
        NormalizationStrategy::AbsoluteBand => {
            // Cosine similarities for embeddings typically range [0.2 .. 0.9]
            raw_dense_values
                .iter()
                .map(|&c| ((c - 0.2) / 0.7).clamp(0.0, 1.0))
                .collect()
        }
        NormalizationStrategy::EmpiricalCdf => {
            let ecdf = EmpiricalCdf::from_samples(raw_dense_values.clone());
            raw_dense_values
                .iter()
                .map(|&c| ecdf.percentile(c))
                .collect()
        }
    };

    for (hit, &norm_sim) in dense_hits.iter().zip(norm_dense.iter()) {
        let key = hit.chunk.chunk_id.as_str();
        chunk_map.insert(key, hit.chunk);
        dense_scores.insert(key, norm_sim);
    }

    // 3. Convex combination with CYNIC altitude and deprecation policy
    let mut fused = Vec::with_capacity(chunk_map.len());
    for (key, &chunk) in &chunk_map {
        let s_bm25 = sparse_scores.get(key).copied().unwrap_or(0.0);
        let s_dense = dense_scores.get(key).copied().unwrap_or(0.0);

        let combined = alpha * s_dense + (1.0 - alpha) * s_bm25;

        let contract_floor = evaluate_contract_floor(&chunk.source_path);
        let altitude = evaluate_altitude_multiplier(chunk.authority);
        let deprecation = evaluate_deprecation_multiplier(&chunk.source_path, &chunk.content);

        let final_score = (combined * 100.0 + contract_floor) * altitude * deprecation;

        fused.push(SearchHit {
            chunk,
            score: final_score,
        });
    }

    // Sort descending by score, breaking ties deterministically by chunk_id
    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
    });

    fused
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::AuthorityTier;

    fn make_test_chunk(id: &str, path: &str) -> DocumentChunk {
        DocumentChunk {
            source_path: path.to_string(),
            line_start: 1,
            line_end: 10,
            content: "content".to_string(),
            chunk_id: id.to_string(),
            authority: AuthorityTier::L1Specification,
            section_hierarchy: vec!["Root".to_string()],
        }
    }

    #[test]
    fn test_min_max_normalization_bounds() {
        let scores = vec![10.0, 20.0, 30.0];
        let norm = min_max_normalize(&scores);
        assert!((norm[0] - 0.0).abs() < 1e-6);
        assert!((norm[1] - 0.5).abs() < 1e-6);
        assert!((norm[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ecdf_percentile_monotonicity() {
        let samples = vec![1.0, 2.0, 5.0, 10.0];
        let ecdf = EmpiricalCdf::from_samples(samples);
        assert_eq!(ecdf.percentile(0.5), 0.0);
        assert_eq!(ecdf.percentile(1.0), 0.25);
        assert_eq!(ecdf.percentile(3.0), 0.50);
        assert_eq!(ecdf.percentile(10.0), 1.0);
    }

    #[test]
    fn test_convex_combination_alpha_extremes() {
        let chunk_a = make_test_chunk("id_a", "contracts/a.json");
        let chunk_b = make_test_chunk("id_b", "contracts/b.json");

        let sparse = vec![
            SearchHit {
                chunk: &chunk_a,
                score: 20.0,
            },
            SearchHit {
                chunk: &chunk_b,
                score: 10.0,
            },
        ];
        let dense = vec![
            DenseHit {
                chunk: &chunk_b,
                similarity: 0.9,
            },
            DenseHit {
                chunk: &chunk_a,
                similarity: 0.1,
            },
        ];

        // alpha = 0.0 (pure BM25): chunk_a must be rank 1
        let fused_sparse =
            fuse_continuous_convex(&sparse, &dense, 0.0, NormalizationStrategy::CandidateMinMax);
        assert_eq!(fused_sparse[0].chunk.chunk_id, "id_a");

        // alpha = 1.0 (pure Dense): chunk_b must be rank 1
        let fused_dense =
            fuse_continuous_convex(&sparse, &dense, 1.0, NormalizationStrategy::CandidateMinMax);
        assert_eq!(fused_dense[0].chunk.chunk_id, "id_b");
    }
}
