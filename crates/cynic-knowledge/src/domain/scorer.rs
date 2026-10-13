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
}
