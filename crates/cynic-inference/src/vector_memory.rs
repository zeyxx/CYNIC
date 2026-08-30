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
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        results
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Indexes an archive JSONL file (such as compacted Askēsis logs) into the semantic store.
    /// For each entry, computes an embedding using the provided embedder function.
    pub fn index_jsonl_archive<F>(
        &mut self,
        path: &std::path::Path,
        mut embedder: F,
    ) -> std::io::Result<ArchiveIndexingReport>
    where
        F: FnMut(&str) -> Embedding,
    {
        use std::io::{BufRead, BufReader};
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut indexed_events = 0;
        let mut skipped_events = 0;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                let content = value
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default();
                if content.trim().is_empty() {
                    skipped_events += 1;
                    continue;
                }
                let timestamp = value.get("timestamp").and_then(|t| t.as_i64()).unwrap_or(0);
                let domain = value
                    .get("domain")
                    .and_then(|d| d.as_str())
                    .unwrap_or("general");
                let hash = value
                    .get("cryptographic_hash")
                    .and_then(|h| h.as_str())
                    .unwrap_or("");

                let id = if !hash.is_empty() {
                    format!("askesis-{hash}")
                } else {
                    format!("askesis-{timestamp}-{indexed_events}")
                };

                let embedding = embedder(content);
                let metadata = serde_json::json!({
                    "source": "askesis_archive",
                    "timestamp": timestamp,
                    "domain": domain,
                    "cryptographic_hash": hash,
                    "file": path.file_name().and_then(|n| n.to_str()).unwrap_or("")
                });

                self.insert(id, content, &embedding, metadata);
                indexed_events += 1;
            } else {
                skipped_events += 1;
            }
        }

        Ok(ArchiveIndexingReport {
            archive_path: path.to_string_lossy().to_string(),
            indexed_events,
            skipped_events,
            total_store_size: self.len(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveIndexingReport {
    pub archive_path: String,
    pub indexed_events: usize,
    pub skipped_events: usize,
    pub total_store_size: usize,
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

        store.insert(
            "crystal-1",
            "Coordination lease rules",
            &emb1,
            serde_json::json!({"kind": "crystal"}),
        );
        store.insert(
            "crystal-2",
            "Solana pump trading rules",
            &emb2,
            serde_json::json!({"kind": "crypto"}),
        );
        store.insert(
            "crystal-3",
            "Agent fencing token verification",
            &emb3,
            serde_json::json!({"kind": "crystal"}),
        );

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

    #[test]
    fn index_jsonl_archive_loads_and_queries_semantic_entries() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let archive_file = temp_dir.path().join("archive-1000-2000.jsonl");
        std::fs::write(
            &archive_file,
            r#"{"timestamp":1000,"domain":"infra","content":"Proxmox hypervisor cluster restarted","cryptographic_hash":"hash1"}
{"timestamp":1050,"domain":"build","content":"Build runner memory floor exceeded","cryptographic_hash":"hash2"}
{"timestamp":1100,"domain":"auth","content":"","cryptographic_hash":"hash3"}
"#,
        )
        .expect("write archive fixture");

        let mut store = VectorMemoryStore::new();
        let fake_embedder = |text: &str| {
            if text.contains("Proxmox") {
                Embedding {
                    vector: vec![1.0, 0.0, 0.0],
                    dimensions: 3,
                    prompt_tokens: 2,
                }
            } else {
                Embedding {
                    vector: vec![0.0, 1.0, 0.0],
                    dimensions: 3,
                    prompt_tokens: 2,
                }
            }
        };

        let report = store
            .index_jsonl_archive(&archive_file, fake_embedder)
            .expect("index archive");

        assert_eq!(report.indexed_events, 2);
        assert_eq!(report.skipped_events, 1); // empty content skipped
        assert_eq!(report.total_store_size, 2);

        let query = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            dimensions: 3,
            prompt_tokens: 1,
        };
        let hits = store.search(&query, 1, 0.9);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "askesis-hash1");
        assert_eq!(hits[0].metadata["domain"], "infra");
    }
}
