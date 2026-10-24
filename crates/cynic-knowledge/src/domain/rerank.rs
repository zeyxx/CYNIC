//! Neural Cross-Encoder Reranker Domain Module
//! Follows CYNIC FCIS: Pure domain logic for reranking candidate search hits.

use std::collections::HashMap;

use crate::domain::scorer::SearchHit;
use serde::{Deserialize, Serialize};

pub const RERANK_CACHE_SCHEMA_V1: &str = "cynic.knowledge.rerank_cache.v1";

/// Persistent cache for cross-encoder reranking scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankCacheRecord {
    pub schema: String,
    pub model: String,
    /// Maps deterministic key `blake3(format!("{query}:{chunk_id}"))` -> relevance_score.
    pub scores: HashMap<String, f64>,
}

impl Default for RerankCacheRecord {
    fn default() -> Self {
        Self::new("default")
    }
}

impl RerankCacheRecord {
    pub fn new(model: &str) -> Self {
        Self {
            schema: RERANK_CACHE_SCHEMA_V1.to_string(),
            model: model.to_string(),
            scores: HashMap::new(),
        }
    }

    pub fn key(query: &str, chunk_id: &str) -> String {
        blake3::hash(format!("{query}:{chunk_id}").as_bytes())
            .to_hex()
            .to_string()
    }

    pub fn get(&self, query: &str, chunk_id: &str) -> Option<f64> {
        let k = Self::key(query, chunk_id);
        self.scores.get(&k).copied()
    }

    pub fn insert(&mut self, query: &str, chunk_id: &str, score: f64) {
        let k = Self::key(query, chunk_id);
        self.scores.insert(k, score);
    }

    pub fn len(&self) -> usize {
        self.scores.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankItem {
    pub index: usize,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub results: Vec<RerankItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankRequest<'a> {
    pub query: &'a str,
    pub documents: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<usize>,
}

/// Pure reranking function: re-orders candidate hits using cross-encoder relevance scores.
pub fn apply_rerank_scores<'a>(
    hits: &[SearchHit<'a>],
    results: &[RerankItem],
) -> Vec<SearchHit<'a>> {
    if hits.is_empty() || results.is_empty() {
        return hits.to_vec();
    }

    let mut reranked = Vec::with_capacity(results.len());
    for item in results {
        if let Some(hit) = hits.get(item.index) {
            reranked.push(SearchHit {
                chunk: hit.chunk,
                score: item.relevance_score,
            });
        }
    }

    // Sort descending by relevance score, tie-breaking deterministically by chunk_id
    reranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
    });

    reranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::{AuthorityTier, DocumentChunk};

    #[test]
    fn test_apply_rerank_reorders_hits() {
        let chunk1 = DocumentChunk::new(
            "file1.md".into(),
            1,
            10,
            vec![],
            "content 1".into(),
            AuthorityTier::L2Design,
        );
        let chunk2 = DocumentChunk::new(
            "file2.md".into(),
            1,
            10,
            vec![],
            "content 2".into(),
            AuthorityTier::L2Design,
        );

        let hits = vec![
            SearchHit {
                chunk: &chunk1,
                score: 0.9,
            },
            SearchHit {
                chunk: &chunk2,
                score: 0.8,
            },
        ];

        let rerank_items = vec![
            RerankItem {
                index: 0,
                relevance_score: 0.2,
            },
            RerankItem {
                index: 1,
                relevance_score: 0.95,
            },
        ];

        let reranked = apply_rerank_scores(&hits, &rerank_items);
        assert_eq!(reranked.len(), 2);
        assert_eq!(reranked[0].chunk.source_path, "file2.md");
        assert_eq!(reranked[1].chunk.source_path, "file1.md");
        assert_eq!(reranked[0].score, 0.95);
    }

    #[test]
    fn test_rerank_cache_insert_and_get() {
        let mut cache = RerankCacheRecord::new("bge-reranker-large");
        assert!(cache.is_empty());

        cache.insert("what is cynic", "chunk_123", 0.985);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("what is cynic", "chunk_123"), Some(0.985));
        assert_eq!(cache.get("other query", "chunk_123"), None);
    }
}
