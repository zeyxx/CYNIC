//! Local Vector Memory — Sovereign Semantic Memory & Top-K Retrieval
//!
//! Provides in-memory and local semantic search over embedded knowledge,
//! crystals, past verdicts, and agent context fragments.

use crate::domain::embedding::Embedding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMemoryEntry {
    pub id: String,
    pub text: String,
    pub dimensions: usize,
    pub vector: Vec<f32>,
    pub metadata: serde_json::Value,
    pub created_at_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub text: String,
    pub similarity: f64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Default, Clone)]
pub struct VectorMemoryStore {
    entries: Vec<VectorMemoryEntry>,
}

impl VectorMemoryStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts a new semantic entry with its embedding.
    pub fn insert(
        &mut self,
        id: impl Into<String>,
        text: impl Into<String>,
        embedding: &Embedding,
        metadata: serde_json::Value,
    ) {
        let created_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        self.entries.push(VectorMemoryEntry {
            id: id.into(),
            text: text.into(),
            dimensions: embedding.dimensions,
            vector: embedding.vector.clone(),
            metadata,
            created_at_unix_ms,
        });
    }

    /// Performs cosine similarity search and returns the top-k nearest semantic matches.
    pub fn search(
        &self,
        query: &Embedding,
        top_k: usize,
        min_similarity: f64,
    ) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let entry_embedding = Embedding {
                    vector: entry.vector.clone(),
                    dimensions: entry.dimensions,
                    prompt_tokens: 0,
                };
                let sim = query.cosine_similarity(&entry_embedding);
                if sim >= min_similarity {
                    Some(SearchResult {
                        id: entry.id.clone(),
                        text: entry.text.clone(),
                        similarity: sim,
                        metadata: entry.metadata.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort descending by similarity score
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_memory_stores_and_retrieves_top_k() {
        let mut store = VectorMemoryStore::new();

        let emb1 = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            dimensions: 3,
            prompt_tokens: 2,
        };
        let emb2 = Embedding {
            vector: vec![0.0, 1.0, 0.0],
            dimensions: 3,
            prompt_tokens: 2,
        };
        let emb3 = Embedding {
            vector: vec![0.9, 0.1, 0.0],
            dimensions: 3,
            prompt_tokens: 2,
        };

        store.insert("crystal-1", "Coordination lease rules", &emb1, serde_json::json!({"kind": "crystal"}));
        store.insert("crystal-2", "Solana pump trading rules", &emb2, serde_json::json!({"kind": "crypto"}));
        store.insert("crystal-3", "Agent fencing token verification", &emb3, serde_json::json!({"kind": "crystal"}));

        let query = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            dimensions: 3,
            prompt_tokens: 1,
        };

        let hits = store.search(&query, 2, 0.5);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "crystal-1");
        assert_eq!(hits[1].id, "crystal-3");
    }
}
