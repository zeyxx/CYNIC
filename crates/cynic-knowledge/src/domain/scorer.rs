use crate::domain::chunk::{AuthorityTier, DocumentChunk};
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

        for (idx, chunk) in self.chunks.iter().enumerate() {
            let term_counts = &self.chunk_term_counts[idx];
            let chunk_len = term_counts.values().sum::<usize>() as f64;

            let mut score = 0.0;
            for token in &query_tokens {
                if let Some(&tf) = term_counts.get(token) {
                    let df = *self.doc_frequencies.get(token).unwrap_or(&0) as f64;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

                    let tf_norm = (tf as f64 * (k1 + 1.0))
                        / (tf as f64 + k1 * (1.0 - b + b * (chunk_len / self.avg_chunk_length)));
                    score += idf * tf_norm;
                }
            }

            if score > 0.0 {
                // Authority tier weighting (L0 constitution gets boost over exploratory notes)
                let authority_multiplier = match chunk.authority {
                    AuthorityTier::L0Constitution => 1.25,
                    AuthorityTier::L1Specification => 1.10,
                    AuthorityTier::L2Design => 1.0,
                    AuthorityTier::L3Operation => 0.90,
                };

                scored_chunks.push(SearchHit {
                    chunk,
                    score: score * authority_multiplier,
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

    /// Returns the total number of indexed chunks.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
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
