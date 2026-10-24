use std::path::Path;

use crate::domain::rerank::RerankCacheRecord;

/// Loads persistent cross-encoder rerank score cache from disk.
pub fn load_rerank_cache<P: AsRef<Path>>(path: P) -> Option<RerankCacheRecord> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Atomically persists cross-encoder rerank score cache to disk.
pub fn save_rerank_cache<P: AsRef<Path>>(
    path: P,
    cache: &RerankCacheRecord,
) -> Result<(), String> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(cache).map_err(|e| e.to_string())?;
    std::fs::write(p, bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_rerank_cache_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("cynic_rerank_cache_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let file_path = temp_dir.join("rerank_cache.json");

        let mut cache = RerankCacheRecord::new("bge-reranker-large");
        cache.insert("query", "chunk1", 0.99);
        save_rerank_cache(&file_path, &cache).expect("save should succeed");

        let loaded = load_rerank_cache(&file_path).expect("load should succeed");
        assert_eq!(loaded.model, "bge-reranker-large");
        assert_eq!(loaded.get("query", "chunk1"), Some(0.99));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
