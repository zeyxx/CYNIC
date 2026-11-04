pub mod adapters;
pub mod domain;
pub mod service;
pub mod vector;

pub mod idea_ledger;

pub use vector::{ContiguousVectorStore, SimdVectorAdapter};

pub use adapters::{
    GitExtractError, categorize_interaction, classify_repo_domain, classify_repo_era,
    extract_git_history, find_canonical_registry_path, git_proof_resolver, harvest_live_ledger,
    load_author_profile, load_authority_overrides, load_canonical_retrieval_policy,
    load_corpus_cache, load_rerank_cache, load_workspace_geometry, local_repo_name,
    parse_document_tree, parse_git_log_output, parse_github_repos, parse_interactions_graph,
    parse_json_contract, parse_json_contract_with_authority, parse_jsonl_wal, parse_markdown_tree,
    parse_rust_file, resolve_service_endpoint_from_registry, save_corpus_cache, save_rerank_cache,
    shell_claim_runner, sync_workspace_corpus, to_surreal_fact_record,
};
pub use domain::{
    AUTHORITY_TIERS_REGISTRY_PATH, AUTHORITY_TIERS_SCHEMA_V1, AuthorIdentity, AuthorProfile,
    AuthorshipNature, AuthorityOverrides, AuthorityTier, BrokenReference,
    CORPUS_CACHE_SCHEMA_V2, CORPUS_MANIFEST_SCHEMA_V1, CachedFileRecord, CalibratedSkillGroup,
    CalibratedSkillItem, ChunkEmbedder, ChunkVector, CitedSource, ClaimCategory, ClaimCheck,
    ClaimCheckOutcome, ClaimStatus, CodeBlock, CorpusCacheRecord, CorpusEdge, CorpusEdgeKind,
    CorpusGraph, CorpusManifest, CrateGeometryRecord, DEFAULT_EPISTEMIC_COVERAGE_THRESHOLD,
    DEFAULT_RRF_K, DOCUMENT_TREE_SCHEMA_V1, DenseHit, DocumentChunk, DocumentTree,
    EPISTEMIC_FACTS_SCHEMA_V1, EpistemicAssemblyConfig, EpistemicCoverageVerdict,
    EpistemicFactLedger, EpistemicSource, GIT_COMMIT_SCHEMA_V1, GitCommitRecord, HistoricalEvent,
    INTERACTIONS_GRAPH_SCHEMA_V1, InteractionCategory, InteractionRecord, InteractionsGraph,
    KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph, LexicalHit, LexicalIndex, MarkdownLink,
    NormalizationStrategy, ProofOutcome, ProofRef, ProofVerdict, QueryRoute, RERANK_CACHE_SCHEMA_V1,
    RETRIEVAL_POLICY_CONTRACT_PATH, RETRIEVAL_POLICY_SCHEMA_V1, RepoEra, RepoProvenance,
    RerankCacheRecord, RerankItem, RerankRequest, RerankResponse, RetrievalPolicy, RetrievalProfile,
    RoutingTable, SYNTHESIS_PROMPT_SCHEMA_V1, SearchHit, SectionNode, SkillBasis, SkillDepth,
    SynthesisPrompt, SynthesisPromptBuilder, TechnicalDomain, UNCOVERED_REJECTION_TOKEN,
    VectorCacheRecord, VerifiableClaim, WORKSPACE_GEOMETRY_SCHEMA_V1, WorkspaceGeometry,
    apply_rerank_scores, assemble_epistemic_context, authority_for, bm25_margin,
    check_claims, check_freshness, cosine_similarity, dense_margin, epoch_seconds,
    expand_hits_with_graph, expand_hits_with_graph_multi_hop, expand_hits_with_graph_query,
    expand_hits_with_graph_query_hops, expand_hits_with_profile,
    expand_hits_with_topological_contexts, fuse_continuous_convex, fuse_reciprocal_ranks,
    hybrid_retrieve, load_vector_cache, normalize_engine, parse_count, parse_proof_reference,
    render_public_surface_html, render_public_surface_markdown, route_query, save_vector_cache,
    simd_top_k_dense, tokenize, top_k_dense, vectorize_chunks, vectorize_chunks_cached,
    verify_event_proofs,
};

pub use idea_ledger::{DEFAULT_LEDGER_STATUS, IDEA_LEDGER_SCHEMA, VALID_FRESHNESSES, VALID_STATUSES,
    IdeaEntry, IdeaLedger, LedgerError, consume, filter_ideas, generate_idea_id, is_valid_freshness,
    is_valid_status, update_freshness, update_status};

// Service module — knowledge command surface (L3)

use std::path::Path;

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

    // 2.5 Scan observation WAL files (.jsonl)
    for entry in WalkDir::new(root).into_iter().flatten() {
        if entry.file_type().is_file()
            && entry.path().extension().is_some_and(|ext| ext == "jsonl")
        {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let rel_path = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();
                let chunks = crate::adapters::json_contract::parse_jsonl_wal(&rel_path, &content);
                all_chunks.extend(chunks);
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
