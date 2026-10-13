pub mod adapters;
pub mod domain;

pub use adapters::{
    GitExtractError, extract_git_history, load_workspace_geometry, parse_document_tree,
    parse_git_log_output, parse_json_contract, parse_markdown_tree,
};
pub use domain::{
    AuthorityTier, BrokenReference, CORPUS_MANIFEST_SCHEMA_V1, CodeBlock, CorpusManifest,
    CrateGeometryRecord, DOCUMENT_TREE_SCHEMA_V1, DocumentChunk, DocumentTree,
    GIT_COMMIT_SCHEMA_V1, GitCommitRecord, KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph,
    LexicalIndex, MarkdownLink, SearchHit, SectionNode, WORKSPACE_GEOMETRY_SCHEMA_V1,
    WorkspaceGeometry,
};

use std::path::Path;
use walkdir::WalkDir;

/// Standard directory exclusion set (never index build artifacts or VCS state).
const IGNORED_DIRS: &[&str] = &[
    "target",
    ".git",
    ".cargo",
    ".venv_data",
    "node_modules",
    "shared-target",
];

/// Scans 100% of Markdown files across the repository and builds a versioned CorpusManifest.
pub fn scan_corpus_manifest<P: AsRef<Path>>(root: P) -> CorpusManifest {
    let root = root.as_ref();
    let mut documents = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !IGNORED_DIRS.contains(&name.as_ref())
        })
        .flatten()
    {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let rel_path = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();
                let doc_tree = parse_document_tree(&rel_path, &content);
                documents.push(doc_tree);
            }
        }
    }

    CorpusManifest::from_documents(documents)
}

/// Scans and indexes all documentation and contracts in a CYNIC workspace for fast lexical search.
pub fn index_workspace<P: AsRef<Path>>(root: P) -> LexicalIndex {
    index_workspace_with_commits(root, 0)
}

/// Scans and indexes documentation, contracts, and optionally recent Git commits into BM25 index.
pub fn index_workspace_with_commits<P: AsRef<Path>>(root: P, max_commits: usize) -> LexicalIndex {
    let root = root.as_ref();
    let mut all_chunks = Vec::new();

    // 1. Scan 100% of repository markdown documents
    let manifest = scan_corpus_manifest(root);
    for doc in manifest.documents {
        if let Ok(content) = std::fs::read_to_string(root.join(&doc.source_path)) {
            let chunks = parse_markdown_tree(&doc.source_path, &content);
            all_chunks.extend(chunks);
        }
    }

    // 2. Scan contracts/ directory
    let contracts_dir = root.join("contracts");
    if contracts_dir.exists() {
        for entry in WalkDir::new(contracts_dir).into_iter().flatten() {
            if entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "json")
            {
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

    // 3. Scan Git commits if requested
    if max_commits > 0 {
        if let Ok(commits) = extract_git_history(root, max_commits) {
            for commit in commits {
                all_chunks.push(commit.to_chunk());
            }
        }
    }

    LexicalIndex::build(all_chunks)
}
