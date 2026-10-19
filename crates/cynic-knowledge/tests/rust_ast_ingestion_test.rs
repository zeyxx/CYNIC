use std::fs;
use std::path::PathBuf;

use cynic_knowledge::{AuthorityTier, sync_workspace_corpus};

fn test_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cynic_rust_ast_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_sync_workspace_corpus_indexes_rust_ast() {
    let dir = test_temp_dir("ast_indexing");
    let root = &dir;

    // Create crates/cynic-types/src/lib.rs
    let rust_dir = root.join("crates/cynic-types/src");
    fs::create_dir_all(&rust_dir).unwrap();
    let rust_file = rust_dir.join("lib.rs");

    let code = r#"//! Sovereign Types Crate L0 Invariant.

/// Canonical paths utility for sovereign file layout.
pub struct CynicPaths {
    pub root: std::path::PathBuf,
}

impl CynicPaths {
    /// Returns the system state directory.
    pub fn state_dir() -> std::path::PathBuf {
        std::path::PathBuf::from("/var/lib/cynic")
    }
}
"#;
    fs::write(&rust_file, code).unwrap();

    let cache_file = root.join("cache.json");
    let (cache, index) = sync_workspace_corpus(root, Some(&cache_file), 0);

    assert_eq!(cache.files.len(), 1);
    assert!(cache.files.contains_key("crates/cynic-types/src/lib.rs"));

    // Search for struct name
    let hits = index.search("CynicPaths", 5);
    assert!(!hits.is_empty(), "must find CynicPaths struct");
    assert_eq!(hits[0].chunk.authority, AuthorityTier::L0Constitution);

    // Search for function documentation
    let hits_fn = index.search("state_dir system", 5);
    assert!(!hits_fn.is_empty(), "must find state_dir function");
    assert!(hits_fn[0].chunk.content.contains("/var/lib/cynic"));
}
