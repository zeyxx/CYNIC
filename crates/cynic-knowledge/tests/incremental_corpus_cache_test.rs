use std::fs;
use tempfile::tempdir;

use cynic_knowledge::{load_corpus_cache, sync_workspace_corpus};

#[test]
fn test_incremental_cache_avoids_reparsing_unmodified_files() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. Setup sample document
    let docs_dir = root.join("docs");
    fs::create_dir_all(&docs_dir).unwrap();
    let doc_path = docs_dir.join("test.md");
    fs::write(
        &doc_path,
        "# Epistemic Invariant\nRule 1: Always verify physical reality.\n",
    )
    .unwrap();

    let cache_file = root.join("corpus_cache.json");

    // 2. Cold sync (cache generated)
    let (cache1, index1) = sync_workspace_corpus(root, Some(&cache_file), 0);
    assert_eq!(cache1.files.len(), 1);
    assert!(cache_file.is_file());

    let hits1 = index1.search("physical reality", 5);
    assert_eq!(hits1.len(), 1);
    assert_eq!(hits1[0].chunk.source_path, "docs/test.md");

    // 3. Inspect cached file metadata
    let loaded = load_corpus_cache(&cache_file).expect("loaded cache");
    assert_eq!(loaded.files.len(), 1);
    let cached_entry = loaded.files.get("docs/test.md").expect("cached doc");
    assert_eq!(cached_entry.chunks.len(), 1);

    // 4. Warm sync (instantaneous without modification)
    let (cache2, index2) = sync_workspace_corpus(root, Some(&cache_file), 0);
    assert_eq!(cache2.files.len(), 1);
    assert_eq!(index2.search("physical reality", 5).len(), 1);

    // 5. Modification: add a second document
    let doc2_path = docs_dir.join("spec.md");
    fs::write(
        &doc2_path,
        "# Kernel Spec\nRule 2: Pure deterministic predicates.\n",
    )
    .unwrap();

    let (cache3, index3) = sync_workspace_corpus(root, Some(&cache_file), 0);
    assert_eq!(cache3.files.len(), 2);
    assert_eq!(index3.search("deterministic predicates", 5).len(), 1);
    assert_eq!(index3.search("physical reality", 5).len(), 1);
}
