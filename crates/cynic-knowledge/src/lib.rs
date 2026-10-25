pub mod adapters;
pub mod domain;

pub use adapters::{
    git_proof_resolver, local_repo_name,
    categorize_interaction, classify_repo_domain, classify_repo_era, extract_git_history, load_author_profile,
    find_canonical_registry_path, harvest_live_ledger, load_authority_overrides,
    load_corpus_cache, load_workspace_geometry, parse_document_tree, parse_git_log_output,
    parse_github_repos, parse_interactions_graph, parse_json_contract,
    parse_json_contract_with_authority, parse_markdown_tree, parse_rust_file,
    resolve_service_endpoint_from_registry, save_corpus_cache, sync_workspace_corpus,
    load_rerank_cache, save_rerank_cache, load_canonical_retrieval_policy, GitExtractError,
};
pub use domain::{
    apply_rerank_scores, assemble_epistemic_context, authority_for, bm25_margin, cosine_similarity, dense_margin,
    expand_hits_with_graph, expand_hits_with_graph_query, fuse_continuous_convex, fuse_reciprocal_ranks, load_vector_cache,
    normalize_engine, render_public_surface_html, render_public_surface_markdown,
    RerankCacheRecord, RerankItem, RerankRequest, RerankResponse, RERANK_CACHE_SCHEMA_V1,
    RetrievalPolicy, RetrievalProfile, RETRIEVAL_POLICY_CONTRACT_PATH, RETRIEVAL_POLICY_SCHEMA_V1,
    route_query, save_vector_cache, tokenize, top_k_dense, vectorize_chunks,
    vectorize_chunks_cached, AuthorIdentity, AuthorProfile, AuthorshipNature, AuthorityOverrides, AuthorityTier, BrokenReference,
    CachedFileRecord, CalibratedSkillGroup, CalibratedSkillItem, ChunkEmbedder, ChunkVector,
    CitedSource, ClaimCategory, ClaimStatus, CodeBlock, CorpusCacheRecord, CorpusManifest,
    CrateGeometryRecord, DenseHit, DocumentChunk, DocumentTree, EpistemicAssemblyConfig,
    EpistemicCoverageVerdict, EpistemicFactLedger, EpistemicSource, GitCommitRecord, HistoricalEvent,
    InteractionCategory, InteractionRecord, InteractionsGraph,
    KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph, LexicalIndex, MarkdownLink,
    NormalizationStrategy, QueryRoute, RepoEra, RepoProvenance, RoutingTable, SearchHit,
    SectionNode, SkillBasis, SkillDepth, SynthesisPrompt, SynthesisPromptBuilder, TechnicalDomain,
    VectorCacheRecord, VerifiableClaim, WorkspaceGeometry, AUTHORITY_TIERS_REGISTRY_PATH,
    AUTHORITY_TIERS_SCHEMA_V1, CORPUS_CACHE_SCHEMA_V2, CORPUS_MANIFEST_SCHEMA_V1, DEFAULT_EPISTEMIC_COVERAGE_THRESHOLD, DEFAULT_RRF_K,
    DOCUMENT_TREE_SCHEMA_V1, EPISTEMIC_FACTS_SCHEMA_V1, GIT_COMMIT_SCHEMA_V1,
    INTERACTIONS_GRAPH_SCHEMA_V1, SYNTHESIS_PROMPT_SCHEMA_V1, UNCOVERED_REJECTION_TOKEN,
    WORKSPACE_GEOMETRY_SCHEMA_V1,
    check_freshness, epoch_seconds, parse_proof_reference, verify_event_proofs, ProofOutcome, ProofRef, ProofVerdict,
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
    let overrides = load_authority_overrides(root);
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
                    let chunks = parse_json_contract_with_authority(
                        &rel_path,
                        &content,
                        authority_for(&rel_path, &overrides),
                    );
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
