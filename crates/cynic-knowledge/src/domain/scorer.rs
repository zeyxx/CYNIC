use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use crate::domain::hybrid_rrf::{DEFAULT_RRF_K, fuse_reciprocal_ranks};
use crate::domain::vectorizer::{ChunkVector, top_k_dense};
use std::collections::HashMap;

/// Pure in-memory BM25 lexical search and ranking engine.
#[derive(Debug, Clone, Default)]
pub struct LexicalIndex {
    chunks: Vec<DocumentChunk>,
    /// Token -> Document frequencies (number of chunks containing token).
    doc_frequencies: HashMap<String, usize>,
    /// Chunk index -> Token counts (term frequencies).
    chunk_term_counts: Vec<HashMap<String, usize>>,
    /// Average chunk length in tokens.
    avg_chunk_length: f64,
}

/// A matched search result with score and proof breadcrumbs.
#[derive(Debug, Clone)]
pub struct SearchHit<'a> {
    pub chunk: &'a DocumentChunk,
    pub score: f64,
}

impl LexicalIndex {
    /// Builds a new index from a collection of DocumentChunks.
    pub fn build(chunks: Vec<DocumentChunk>) -> Self {
        let n_chunks = chunks.len();
        if n_chunks == 0 {
            return Self::default();
        }

        let mut doc_frequencies = HashMap::new();
        let mut chunk_term_counts = Vec::with_capacity(n_chunks);
        let mut total_tokens = 0;

        for chunk in &chunks {
            let tokens = tokenize(&chunk.content);
            total_tokens += tokens.len();

            let mut counts = HashMap::new();
            for token in tokens {
                *counts.entry(token).or_insert(0) += 1;
            }

            for token in counts.keys() {
                *doc_frequencies.entry(token.clone()).or_insert(0) += 1;
            }

            chunk_term_counts.push(counts);
        }

        let avg_chunk_length = total_tokens as f64 / n_chunks as f64;

        Self {
            chunks,
            doc_frequencies,
            chunk_term_counts,
            avg_chunk_length,
        }
    }

    /// Searches the index with a raw query using BM25 with authority tier weighting.
    pub fn search<'a>(&'a self, query: &str, limit: usize) -> Vec<SearchHit<'a>> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() || self.chunks.is_empty() {
            return Vec::new();
        }

        let k1 = 1.2;
        let b = 0.75;
        let n = self.chunks.len() as f64;

        let mut scored_chunks = Vec::with_capacity(self.chunks.len());

        let is_technical = is_technical_code_query(query);

        for (idx, chunk) in self.chunks.iter().enumerate() {
            let term_counts = &self.chunk_term_counts[idx];
            let chunk_len = term_counts.values().sum::<usize>() as f64;

            let mut raw_bm25 = 0.0;
            for token in &query_tokens {
                if let Some(&tf) = term_counts.get(token) {
                    let df = *self.doc_frequencies.get(token).unwrap_or(&0) as f64;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

                    let tf_norm = (tf as f64 * (k1 + 1.0))
                        / (tf as f64 + k1 * (1.0 - b + b * (chunk_len / self.avg_chunk_length)));
                    raw_bm25 += idf * tf_norm;
                }
            }

            if raw_bm25 > 0.0 {
                // Jalon 11.4: Matrix Re-Ranking Engine
                // Formula: [BM25 + ContractFloor] * Altitude * Deprecation * QueryModality
                let contract_floor = evaluate_contract_floor(&chunk.source_path);
                let altitude = evaluate_altitude_multiplier(chunk.authority);
                let deprecation =
                    evaluate_deprecation_multiplier(&chunk.source_path, &chunk.content);

                let modality_boost = if !is_technical
                    && (chunk.source_path.ends_with(".md") || chunk.source_path.ends_with(".json"))
                {
                    1.25 // Conceptual questions favor authoritative prose and contracts over raw code functions
                } else {
                    1.0
                };

                let final_score =
                    (raw_bm25 + contract_floor) * altitude * deprecation * modality_boost;

                scored_chunks.push(SearchHit {
                    chunk,
                    score: final_score,
                });
            }
        }

        scored_chunks.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored_chunks.truncate(limit);
        scored_chunks
    }

    /// Hybrid BM25 + dense retrieval fused by RRF. `search` stays the
    /// lexical-only path; this is the additive hybrid entry. Queries with no
    /// lexical token hits still return fused dense hits — that is the case
    /// dense retrieval exists for.
    pub fn search_hybrid<'a>(
        &'a self,
        query: &str,
        query_vector: &[f32],
        vectors: &[ChunkVector],
        limit: usize,
        dense_k: usize,
    ) -> Vec<SearchHit<'a>> {
        self.search_hybrid_guarded(query, query_vector, vectors, limit, dense_k, None)
    }

    /// Guarded hybrid: when BM25 is confident (relative top1-top2 margin at
    /// or above `guard_gap`), dense noise cannot demote the lexical winner
    /// and the sparse ranking returns alone. `None` disables the guard
    /// (plain RRF). Fewer than 2 sparse hits means no margin to assess, so
    /// those queries always fuse — recall first when lexical is thin.
    pub fn search_hybrid_guarded<'a>(
        &'a self,
        query: &str,
        query_vector: &[f32],
        vectors: &[ChunkVector],
        limit: usize,
        dense_k: usize,
        guard_gap: Option<f64>,
    ) -> Vec<SearchHit<'a>> {
        let sparse = self.search(query, limit);
        if let Some(gap) = guard_gap {
            if bm25_margin(&sparse) >= gap {
                return sparse;
            }
        }
        let dense = top_k_dense(query_vector, &self.chunks, vectors, dense_k);
        let mut fused = fuse_reciprocal_ranks(&sparse, &dense, DEFAULT_RRF_K);
        fused.truncate(limit);
        fused
    }

    /// Returns the total number of indexed chunks.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

/// Relative BM25 confidence margin: (top1 - top2) / top1. Zero when the top
/// score is null or when fewer than 2 hits exist — no evidence, no claim.
pub fn bm25_margin(hits: &[SearchHit<'_>]) -> f64 {
    match hits {
        [first, second, ..] if first.score > 0.0 => {
            ((first.score - second.score) / first.score).max(0.0)
        }
        _ => 0.0,
    }
}

/// Extracts alphanumeric normalized tokens, retaining snake_case and kebab-case technical terms.
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|t| t.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

/// Evaluates topological altitude multiplier based on CYNIC authority cone.
pub fn evaluate_altitude_multiplier(authority: AuthorityTier) -> f64 {
    match authority {
        AuthorityTier::L0Constitution => 3.0,
        AuthorityTier::L1Specification => 2.0,
        AuthorityTier::L2Design => 1.3,
        AuthorityTier::L3Operation => 1.0,
    }
}

/// Evaluates deprecation multiplier based on explicit markers in path or content.
pub fn evaluate_deprecation_multiplier(path: &str, content: &str) -> f64 {
    let lower_content = content.to_lowercase();
    let lower_path = path.to_lowercase();
    if lower_content.contains("\"status\": \"superseded\"")
        || lower_content.contains("\"status\": \"deprecated\"")
        || lower_content.contains("\"status\": \"archived\"")
        || lower_content.contains("[superseded]")
        || lower_content.contains("[deprecated]")
        || lower_path.contains("superseded")
        || lower_path.contains("archive")
    {
        0.2 // Strong penalty for obsolete/deprecated material
    } else {
        1.0
    }
}

/// Evaluates contract floor bonus to protect brief normative JSON clauses.
pub fn evaluate_contract_floor(path: &str) -> f64 {
    if path.starts_with("contracts/") || path.ends_with(".json") {
        0.5
    } else {
        0.0
    }
}

/// Detects if a query contains explicit programming symbols (snake_case, PascalCase, or code keywords).
pub fn is_technical_code_query(query: &str) -> bool {
    query.contains('_')
        || query.contains("::")
        || query.starts_with("fn ")
        || query.starts_with("pub ")
        || query.starts_with("struct ")
        || query.starts_with("enum ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bm25_margin_is_relative_gap_and_zero_without_evidence() {
        let chunk = |name: &str| {
            DocumentChunk::new(
                name.into(),
                1,
                2,
                Vec::new(),
                "body".into(),
                AuthorityTier::L3Operation,
            )
        };
        let (a, b, c, d) = (chunk("a.md"), chunk("b.md"), chunk("c.md"), chunk("d.md"));
        let hits = vec![
            SearchHit {
                chunk: &a,
                score: 10.0,
            },
            SearchHit {
                chunk: &b,
                score: 6.0,
            },
        ];
        assert!((bm25_margin(&hits) - 0.4).abs() < 1e-9);
        assert_eq!(bm25_margin(&hits[..1]), 0.0);
        assert_eq!(bm25_margin(&[]), 0.0);
        let zeros = vec![
            SearchHit {
                chunk: &c,
                score: 0.0,
            },
            SearchHit {
                chunk: &d,
                score: 0.0,
            },
        ];
        assert_eq!(bm25_margin(&zeros), 0.0);
    }

    #[test]
    fn test_matrix_reranker_prioritizes_l0_constitution_over_l3_operation() {
        let chunk_l3 = DocumentChunk::new(
            "crates/cynic-cli/src/main.rs".into(),
            1,
            10,
            vec!["Main".into()],
            "system state verification routine".into(),
            AuthorityTier::L3Operation,
        );

        let chunk_l0 = DocumentChunk::new(
            "docs/foundation/CONSTITUTION.md".into(),
            1,
            10,
            vec!["Constitution".into()],
            "system state verification invariant rule".into(),
            AuthorityTier::L0Constitution,
        );

        let index = LexicalIndex::build(vec![chunk_l3, chunk_l0]);
        let hits = index.search("system state verification", 2);

        assert_eq!(hits.len(), 2);
        // L0 Constitution must decisively outrank L3 Operation
        assert_eq!(hits[0].chunk.authority, AuthorityTier::L0Constitution);
        assert!(hits[0].score > hits[1].score * 2.0);
    }

    #[test]
    fn test_deprecated_documents_are_severely_penalized() {
        let active_chunk = DocumentChunk::new(
            "docs/specs/active.md".into(),
            1,
            10,
            vec!["Active".into()],
            "feature execution pipeline".into(),
            AuthorityTier::L1Specification,
        );

        let deprecated_chunk = DocumentChunk::new(
            "docs/specs/old.md".into(),
            1,
            10,
            vec!["Old".into()],
            "feature execution pipeline [SUPERSEDED]".into(),
            AuthorityTier::L1Specification,
        );

        let index = LexicalIndex::build(vec![active_chunk, deprecated_chunk]);
        let hits = index.search("feature execution pipeline", 2);

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].chunk.source_path, "docs/specs/active.md");
        assert!(hits[0].score > hits[1].score * 4.0);
    }

    struct FixedEmbedder(Vec<Vec<f32>>);
    impl crate::domain::vectorizer::ChunkEmbedder for FixedEmbedder {
        fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            assert_eq!(texts.len(), self.0.len(), "one vector per chunk");
            Ok(self.0.clone())
        }
    }

    fn hybrid_chunks() -> Vec<DocumentChunk> {
        vec![
            DocumentChunk::new(
                "docs/alpha.md".into(),
                1,
                3,
                vec![],
                "alpha alpha alpha".into(),
                AuthorityTier::L3Operation,
            ),
            DocumentChunk::new(
                "docs/beta.md".into(),
                1,
                3,
                vec![],
                "beta beta beta".into(),
                AuthorityTier::L3Operation,
            ),
        ]
    }

    #[test]
    fn hybrid_returns_dense_hits_when_lexical_misses_everything() {
        let chunks = hybrid_chunks();
        let index = LexicalIndex::build(chunks.clone());
        // Lexical query shares no token with any chunk.
        assert!(index.search("zzzqqq", 5).is_empty());
        let vectors = crate::domain::vectorizer::vectorize_chunks(
            &FixedEmbedder(vec![vec![1.0, 0.0], vec![0.0, 1.0]]),
            &chunks,
        )
        .expect("vectors");
        let hits = index.search_hybrid("zzzqqq", &[1.0, 0.0], &vectors, 5, 5);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].chunk.source_path, "docs/alpha.md");
    }

    #[test]
    fn hybrid_fuses_both_rankings_and_respects_limit() {
        let chunks = hybrid_chunks();
        let index = LexicalIndex::build(chunks.clone());
        // Lexical ranks beta first; dense ranks alpha first.
        let vectors = crate::domain::vectorizer::vectorize_chunks(
            &FixedEmbedder(vec![vec![1.0, 0.0], vec![0.0, 1.0]]),
            &chunks,
        )
        .expect("vectors");
        let hits = index.search_hybrid("beta", &[1.0, 0.0], &vectors, 1, 5);
        assert_eq!(hits.len(), 1);
        let both = index.search_hybrid("beta", &[1.0, 0.0], &vectors, 5, 5);
        assert_eq!(both.len(), 2);
    }
}
