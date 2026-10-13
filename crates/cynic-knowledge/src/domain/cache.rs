use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::chunk::DocumentChunk;

pub const CORPUS_CACHE_SCHEMA_V1: &str = "cynic.knowledge.corpus_cache.v1";

/// A single cached file containing its physical signature and pre-computed chunks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedFileRecord {
    /// Relative path from repository root (e.g. "docs/foundation/CONSTITUTION.md").
    pub rel_path: String,
    /// Last modification time in epoch seconds.
    pub mtime_secs: u64,
    /// File size in bytes.
    pub size_bytes: u64,
    /// BLAKE3 content hash.
    pub content_hash: String,
    /// Extracted document chunks.
    pub chunks: Vec<DocumentChunk>,
}

/// Persistent, incrementally synchronizable corpus cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusCacheRecord {
    pub schema: String,
    pub generated_at_secs: u64,
    pub head_commit: Option<String>,
    pub files: HashMap<String, CachedFileRecord>,
}

impl CorpusCacheRecord {
    pub fn new(head_commit: Option<String>) -> Self {
        Self {
            schema: CORPUS_CACHE_SCHEMA_V1.to_string(),
            generated_at_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            head_commit,
            files: HashMap::new(),
        }
    }

    /// Returns true if a cached file entry matches physical disk metadata.
    pub fn is_file_fresh(&self, rel_path: &str, mtime_secs: u64, size_bytes: u64) -> bool {
        if let Some(entry) = self.files.get(rel_path) {
            entry.mtime_secs == mtime_secs && entry.size_bytes == size_bytes
        } else {
            false
        }
    }

    /// Inserts or updates a cached file entry.
    pub fn update_file(&mut self, entry: CachedFileRecord) {
        self.files.insert(entry.rel_path.clone(), entry);
    }

    /// Removes files that no longer exist on disk.
    pub fn prune_missing(&mut self, active_paths: &[String]) {
        self.files.retain(|path, _| active_paths.contains(path));
    }

    /// Collects all document chunks across all cached files.
    pub fn collect_all_chunks(&self) -> Vec<DocumentChunk> {
        let mut all = Vec::new();
        for file in self.files.values() {
            all.extend(file.chunks.clone());
        }
        all
    }

    /// Total count of indexed chunks.
    pub fn total_chunks(&self) -> usize {
        self.files.values().map(|f| f.chunks.len()).sum()
    }

    /// Returns a map of relative path to its document chunks.
    pub fn chunks_by_path(&self) -> HashMap<String, Vec<DocumentChunk>> {
        self.files
            .iter()
            .map(|(path, entry)| (path.clone(), entry.chunks.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::AuthorityTier;

    #[test]
    fn test_cache_freshness_and_chunk_collection() {
        let mut cache = CorpusCacheRecord::new(Some("abcdef".into()));
        let chunk = DocumentChunk::new(
            "docs/test.md".into(),
            1,
            10,
            vec!["# Test".into()],
            "Content".into(),
            AuthorityTier::L1Specification,
        );

        let file_rec = CachedFileRecord {
            rel_path: "docs/test.md".into(),
            mtime_secs: 1000,
            size_bytes: 42,
            content_hash: "hash123".into(),
            chunks: vec![chunk],
        };

        cache.update_file(file_rec);

        assert!(cache.is_file_fresh("docs/test.md", 1000, 42));
        assert!(!cache.is_file_fresh("docs/test.md", 1001, 42)); // mtime modified
        assert!(!cache.is_file_fresh("docs/test.md", 1000, 43)); // size modified
        assert!(!cache.is_file_fresh("docs/other.md", 1000, 42)); // absent

        assert_eq!(cache.total_chunks(), 1);
        let chunks = cache.collect_all_chunks();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].source_path, "docs/test.md");

        // Pruning absent files
        cache.prune_missing(&["docs/retained.md".into()]);
        assert_eq!(cache.total_chunks(), 0);
    }
}
