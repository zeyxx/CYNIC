use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

use crate::adapters::git::extract_git_history;
use crate::adapters::json_contract::parse_json_contract;
use crate::adapters::markdown::parse_markdown_tree;
use crate::domain::cache::{CachedFileRecord, CorpusCacheRecord};
use crate::domain::scorer::LexicalIndex;

/// Standard directory exclusion set (never index build artifacts or VCS state).
const IGNORED_DIRS: &[&str] = &[
    "target",
    ".git",
    ".cargo",
    ".venv_data",
    "node_modules",
    "shared-target",
];

/// Loads a corpus cache from a JSON file path on disk.
pub fn load_corpus_cache<P: AsRef<Path>>(path: P) -> Option<CorpusCacheRecord> {
    let p = path.as_ref();
    if !p.is_file() {
        return None;
    }
    let data = fs::read_to_string(p).ok()?;
    serde_json::from_str(&data).ok()
}

/// Saves a corpus cache to a JSON file path atomically via temporary sibling.
pub fn save_corpus_cache<P: AsRef<Path>>(
    path: P,
    cache: &CorpusCacheRecord,
) -> std::io::Result<()> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = serde_json::to_string_pretty(cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let tmp_path = p.with_extension("tmp");
    fs::write(&tmp_path, serialized)?;
    fs::rename(tmp_path, p)
}

/// Incrementally synchronizes the workspace documentation and contracts.
///
/// Uses physical file metadata (mtime + size) to skip re-reading and re-chunking
/// unchanged files. Updates the cache and constructs a fresh BM25 `LexicalIndex`.
pub fn sync_workspace_corpus<P: AsRef<Path>>(
    root: P,
    cache_path: Option<&Path>,
    max_commits: usize,
) -> (CorpusCacheRecord, LexicalIndex) {
    let root = root.as_ref();
    let mut cache = cache_path
        .and_then(load_corpus_cache)
        .unwrap_or_else(|| CorpusCacheRecord::new(None));

    let mut active_paths = Vec::new();
    let mut cache_dirty = false;

    // 1. Scan Markdown documentation files
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !IGNORED_DIRS.contains(&name.as_ref())
        })
        .flatten()
    {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            let rel_path = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();

            active_paths.push(rel_path.clone());

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size_bytes = metadata.len();
            let mtime_secs = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if !cache.is_file_fresh(&rel_path, mtime_secs, size_bytes) {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
                    let chunks = parse_markdown_tree(&rel_path, &content);
                    cache.update_file(CachedFileRecord {
                        rel_path,
                        mtime_secs,
                        size_bytes,
                        content_hash: hash,
                        chunks,
                    });
                    cache_dirty = true;
                }
            }
        }
    }

    // 2. Scan contracts/ directory JSON contracts
    let contracts_dir = root.join("contracts");
    if contracts_dir.exists() {
        for entry in WalkDir::new(contracts_dir).into_iter().flatten() {
            if entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "json")
            {
                let rel_path = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();

                active_paths.push(rel_path.clone());

                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let size_bytes = metadata.len();
                let mtime_secs = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if !cache.is_file_fresh(&rel_path, mtime_secs, size_bytes) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
                        let chunks = parse_json_contract(&rel_path, &content);
                        cache.update_file(CachedFileRecord {
                            rel_path,
                            mtime_secs,
                            size_bytes,
                            content_hash: hash,
                            chunks,
                        });
                        cache_dirty = true;
                    }
                }
            }
        }
    }

    // 3. Prune files deleted from disk
    let initial_count = cache.files.len();
    cache.prune_missing(&active_paths);
    if cache.files.len() != initial_count {
        cache_dirty = true;
    }

    // 4. Persist updated cache if modified
    if cache_dirty {
        if let Some(cp) = cache_path {
            let _ = save_corpus_cache(cp, &cache);
        }
    }

    // 5. Build LexicalIndex
    let mut all_chunks = cache.collect_all_chunks();

    // 6. Include recent Git commits if requested
    if max_commits > 0 {
        if let Ok(commits) = extract_git_history(root, max_commits) {
            for commit in commits {
                all_chunks.push(commit.to_chunk());
            }
        }
    }

    let index = LexicalIndex::build(all_chunks);
    (cache, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cynic_corpus_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_sync_workspace_corpus_incremental_roundtrip() {
        let dir = test_temp_dir("roundtrip");
        let root = &dir;

        // Create sample markdown file
        let doc1 = root.join("doc1.md");
        fs::write(&doc1, "# Title 1\nContent of document 1.\n").unwrap();

        let cache_file = root.join("cache.json");

        // Initial sync: cold cache
        let (cache1, index1) = sync_workspace_corpus(root, Some(&cache_file), 0);
        assert_eq!(cache1.files.len(), 1);
        assert!(cache_file.is_file());

        let hits = index1.search("Title 1", 5);
        assert_eq!(hits.len(), 1);

        // Second sync: warm cache (no modification)
        let (cache2, index2) = sync_workspace_corpus(root, Some(&cache_file), 0);
        assert_eq!(cache2.files.len(), 1);
        assert_eq!(index2.search("Title 1", 5).len(), 1);

        // Modification: add second document
        let doc2 = root.join("doc2.md");
        fs::write(&doc2, "# SpecialHeader\nUnique keyword for document two.\n").unwrap();

        let (cache3, index3) = sync_workspace_corpus(root, Some(&cache_file), 0);
        assert_eq!(cache3.files.len(), 2);
        assert_eq!(index3.search("Unique keyword", 5).len(), 1);
    }
}
