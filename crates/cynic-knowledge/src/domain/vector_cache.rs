//! Persistent dense-vector cache: embed once, resume forever.
//!
//! Vectors are keyed by the content-addressed BLAKE3 `chunk_id`, so edited
//! content naturally misses and gets re-embedded. The record is scoped by
//! model name — vectors from another model are never reused. Storage mirrors
//! the corpus cache (JSON beside it); I/O stays in free functions so the
//! orchestration below remains unit-testable without a filesystem.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::chunk::DocumentChunk;
use crate::domain::vectorizer::{vectorize_chunks, ChunkEmbedder, ChunkVector};

pub const VECTOR_CACHE_SCHEMA_V1: &str = "cynic.knowledge.vector_cache.v1";

/// Vectors for one embedding model, keyed by content-addressed chunk id.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorCacheRecord {
    pub schema: String,
    pub model: String,
    /// Text pipeline that produced these vectors (truncation, instruction
    /// prefixes): any pipeline change invalidates, since equal chunk ids
    /// would otherwise serve vectors from different input texts.
    pub pipeline: String,
    /// Engine endpoint that produced these vectors (normalized base URL).
    /// Two engines serving the same model file still diverge numerically
    /// (measured 2.5e-3 across Vulkan drivers/GPUs), so a cache is bound
    /// to exactly one engine — never silently mixed. Older cache files
    /// predate this field and deserialize it as empty (never usable until
    /// re-stamped), so the upgrade path fails closed.
    #[serde(default)]
    pub engine: String,
    pub dim: usize,
    pub vectors: HashMap<String, Vec<f32>>,
}

/// Normalize an embedding endpoint into a stable engine identity:
/// lowercase host, no trailing slash.
pub fn normalize_engine(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_lowercase()
}

impl VectorCacheRecord {
    pub fn new(model: &str, pipeline: &str, engine: &str) -> Self {
        Self {
            schema: VECTOR_CACHE_SCHEMA_V1.to_string(),
            model: model.to_string(),
            pipeline: pipeline.to_string(),
            engine: normalize_engine(engine),
            dim: 0,
            vectors: HashMap::new(),
        }
    }

    /// A record is usable only for its own model, pipeline, engine, and
    /// schema; anything else means recompute, never silent mixing.
    pub fn usable_for(&self, model: &str, pipeline: &str, engine: &str) -> bool {
        self.schema == VECTOR_CACHE_SCHEMA_V1
            && self.model == model
            && self.pipeline == pipeline
            && self.engine == normalize_engine(engine)
    }
}

/// Load a cache file; `None` on missing file, corrupt JSON, or model/schema/
/// pipeline/engine mismatch — every case means "recompute", never "fail".
pub fn load_vector_cache(
    path: &Path,
    model: &str,
    pipeline: &str,
    engine: &str,
) -> Option<VectorCacheRecord> {
    let raw = std::fs::read_to_string(path).ok()?;
    let record: VectorCacheRecord = serde_json::from_str(&raw).ok()?;
    record.usable_for(model, pipeline, engine).then_some(record)
}

/// Persist the record; errors are reported, never hidden, so a silent
/// no-cache regression cannot masquerade as a cache hit rate of zero.
pub fn save_vector_cache(path: &Path, record: &VectorCacheRecord) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("vector cache mkdir: {error}"))?;
        }
    }
    let raw =
        serde_json::to_string(record).map_err(|error| format!("vector cache encode: {error}"))?;
    std::fs::write(path, raw).map_err(|error| format!("vector cache write: {error}"))
}

/// Embed only chunks missing from `cache`, reuse the rest, and extend the
/// cache in place — the caller saves. Returns `(vectors, reused, embedded)`.
/// Dimension drift inside one run fails closed: mixed-dimension indexes
/// corrupt cosine silently, so refuse instead.
pub fn vectorize_chunks_cached(
    embedder: &dyn ChunkEmbedder,
    chunks: &[DocumentChunk],
    cache: &mut VectorCacheRecord,
    report: &dyn Fn(usize, usize),
) -> Result<(Vec<ChunkVector>, usize, usize), String> {
    let missing: Vec<DocumentChunk> = chunks
        .iter()
        .filter(|chunk| !cache.vectors.contains_key(&chunk.chunk_id))
        .cloned()
        .collect();
    let reused = chunks.len() - missing.len();
    if !missing.is_empty() {
        for fresh in vectorize_chunks(embedder, &missing)? {
            cache.vectors.insert(fresh.chunk_id, fresh.vector);
        }
    }
    let mut dim = cache.dim;
    for vector in cache.vectors.values() {
        if dim == 0 {
            dim = vector.len();
        } else if vector.len() != dim {
            return Err(format!(
                "vector dimension drift: expected {dim}, got {}",
                vector.len()
            ));
        }
    }
    cache.dim = dim;
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let vector = cache
            .vectors
            .get(&chunk.chunk_id)
            .cloned()
            .ok_or_else(|| format!("vector missing after embed: {}", chunk.chunk_id))?;
        out.push(ChunkVector {
            chunk_id: chunk.chunk_id.clone(),
            vector,
        });
    }
    let embedded = chunks.len() - reused;
    report(chunks.len(), chunks.len());
    Ok((out, reused, embedded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::AuthorityTier;

    struct CountingEmbedder(std::cell::Cell<usize>);

    impl ChunkEmbedder for CountingEmbedder {
        fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            self.0.set(self.0.get() + texts.len());
            Ok(texts.iter().map(|_| vec![1.0, 0.0]).collect())
        }
    }

    fn chunk_at(path: &str, content: &str) -> DocumentChunk {
        DocumentChunk::new(
            path.into(),
            1,
            10,
            vec!["# T".into()],
            content.into(),
            AuthorityTier::L3Operation,
        )
    }

    #[test]
    fn cached_run_embeds_only_missing_and_reuses() {
        let embedder = CountingEmbedder(std::cell::Cell::new(0));
        let mut cache = VectorCacheRecord::new("model-a", "pipe-a", "http://engine-a/v1");
        let chunks = vec![chunk_at("a.md", "alpha"), chunk_at("b.md", "beta")];
        let (first, reused, embedded) =
            vectorize_chunks_cached(&embedder, &chunks, &mut cache, &|_, _| {})
                .expect("first run embeds");
        assert_eq!((reused, embedded), (0, 2));
        assert_eq!(first.len(), 2);
        assert_eq!(embedder.0.get(), 2);

        let (second, reused, embedded) =
            vectorize_chunks_cached(&embedder, &chunks, &mut cache, &|_, _| {})
                .expect("second run reuses");
        assert_eq!((reused, embedded), (2, 0));
        assert_eq!(second.len(), 2);
        assert_eq!(embedder.0.get(), 2);
        assert_eq!(first[0].chunk_id, second[0].chunk_id);
    }

    #[test]
    fn edited_content_misses_and_model_scope_rejects() {
        let embedder = CountingEmbedder(std::cell::Cell::new(0));
        let mut cache = VectorCacheRecord::new("model-a", "pipe-a", "http://engine-a/v1");
        let before = vec![chunk_at("a.md", "alpha")];
        vectorize_chunks_cached(&embedder, &before, &mut cache, &|_, _| {}).expect("seed");
        let after = vec![chunk_at("a.md", "alpha edited")];
        let (_, reused, embedded) =
            vectorize_chunks_cached(&embedder, &after, &mut cache, &|_, _| {})
                .expect("edited content re-embeds");
        assert_eq!((reused, embedded), (0, 1));

        assert!(cache.usable_for("model-a", "pipe-a", "http://engine-a/v1"));
        assert!(!cache.usable_for("model-b", "pipe-a", "http://engine-a/v1"));
        assert!(!cache.usable_for("model-a", "pipe-b", "http://engine-a/v1"));
        assert!(!cache.usable_for("model-a", "pipe-a", "http://engine-b/v1"));
    }
}
