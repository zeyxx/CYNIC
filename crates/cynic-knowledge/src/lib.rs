pub mod adapters;
pub mod domain;

pub use adapters::{parse_json_contract, parse_markdown_tree};
pub use domain::{AuthorityTier, DocumentChunk, LexicalIndex, SearchHit};

use std::path::Path;
use walkdir::WalkDir;

/// Scans and indexes all documentation and contracts in a CYNIC workspace.
pub fn index_workspace<P: AsRef<Path>>(root: P) -> LexicalIndex {
    let root = root.as_ref();
    let mut all_chunks = Vec::new();

    // 1. Scan docs/ directory
    let docs_dir = root.join("docs");
    if docs_dir.exists() {
        for entry in WalkDir::new(docs_dir).into_iter().flatten() {
            if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    let rel_path = entry
                        .path()
                        .strip_prefix(root)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .to_string();
                    let chunks = parse_markdown_tree(&rel_path, &content);
                    all_chunks.extend(chunks);
                }
            }
        }
    }

    // 2. Scan root markdown files (AGENTS.md, etc.)
    for entry in WalkDir::new(root).max_depth(1).into_iter().flatten() {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let rel_path = entry.file_name().to_string_lossy().to_string();
                let chunks = parse_markdown_tree(&rel_path, &content);
                all_chunks.extend(chunks);
            }
        }
    }

    // 3. Scan contracts/ directory
    let contracts_dir = root.join("contracts");
    if contracts_dir.exists() {
        for entry in WalkDir::new(contracts_dir).into_iter().flatten() {
            if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    let rel_path = entry
                        .path()
                        .strip_prefix(root)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .to_string();
                    let chunks = parse_json_contract(&rel_path, &content);
                    all_chunks.extend(chunks);
                }
            }
        }
    }

    LexicalIndex::build(all_chunks)
}
