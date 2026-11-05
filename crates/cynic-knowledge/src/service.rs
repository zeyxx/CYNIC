//! Knowledge domain service — the `cynic know` command surface (L3).
//!
//! This module extracts the domain orchestration logic previously located in
//! `cynic-cli/src/knowledge_surface.rs`. It operates on domain types exposed by
//! the `cynic-knowledge` crate and returns structured results; CLI argument
//! parsing and stdout/JSON rendering stay in `cynic-cli`.

use crate::adapters::*;
use crate::domain::*;
use crate::index_workspace;
use std::path::Path;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ServiceError {
    Io(std::io::Error),
    InvalidInput(String),
    NoInferenceEndpoint,
    Inference(String),
}

impl ServiceError {
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        ServiceError::InvalidInput(msg.into())
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::Io(err)
    }
}

// ---------------------------------------------------------------------------
// Index & manifest
// ---------------------------------------------------------------------------

pub struct IndexSummary {
    pub chunk_count: usize,
    pub status: String,
}

pub fn summarize_index(root: &Path) -> Result<IndexSummary, ServiceError> {
    let index = index_workspace(root);
    Ok(IndexSummary {
        chunk_count: index.chunk_count(),
        status: "indexed".to_string(),
    })
}

pub fn scan_corpus_manifest(root: &Path) -> Result<CorpusManifest, ServiceError> {
    Ok(super::scan_corpus_manifest(root))
}

// ---------------------------------------------------------------------------
// Knowledge graph
// ---------------------------------------------------------------------------

pub fn build_knowledge_graph(root: &Path) -> Result<KnowledgeGraph, ServiceError> {
    let manifest = scan_corpus_manifest(root)?;
    let graph = KnowledgeGraph::build_from_manifest(root, &manifest);
    Ok(graph)
}

// ---------------------------------------------------------------------------
// Search, commits, crates
// ---------------------------------------------------------------------------

pub fn search<'a>(
    index: &'a LexicalIndex,
    query: &str,
    limit: usize,
) -> Vec<SearchHit<'a>> {
    index.search(query, limit)
}

pub fn extract_git_history(root: &Path, limit: usize) -> Result<Vec<GitCommitRecord>, ServiceError> {
    extract_git_history(root, limit).map_err(|e| ServiceError::Io(e))
}

pub fn load_crate_geometry(root: &Path) -> Result<WorkspaceGeometry, ServiceError> {
    load_workspace_geometry(root).map_err(ServiceError::Io)
}

// ---------------------------------------------------------------------------
// Ask (synthesis + dense retrieval)
// ---------------------------------------------------------------------------

/// Resolve the cache path for knowledge retrieval cache.
/// Uses the same logic as the CLI: prefer system cache path, fall back to user cache.
fn resolve_knowledge_cache_path() -> std::path::PathBuf {
    // Use user-level cache path by default
    std::env::var("HOME")
        .map(|home| std::path::PathBuf::from(home).join(".cache/cynic/knowledge.cache"))
        .unwrap_or_else(|_| std::path::PathBuf::from("knowledge.cache"))
}

pub struct AskResult<'a> {
    pub endpoint: String,
    pub prompt: SynthesisPrompt,
    pub sources: Vec<CitedSource>,
    pub dense_chunks: Vec<DocumentChunk>,
    pub dense_vectors: Vec<ChunkVector>,
    pub dense_query: Vec<f32>,
    pub fusion: Vec<SearchHit<'a>>,
}

pub fn run_ask<'a>(
    repo_root: &'a Path,
    question: &str,
    limit: usize,
    commits: usize,
    prompt_only: bool,
    endpoint: Option<String>,
    model: String,
) -> Result<AskResult<'a>, ServiceError> {
    let cache_path = resolve_knowledge_cache_path();
    let (cache, index) = sync_workspace_corpus(repo_root, Some(&cache_path), commits);

    // Dense retrieval (delegating to CLI which owns the L2 inference crate).
    // The service exposes the interface; the CLI owns the heavy lifting.
    let cache_chunks = cache.chunks_by_path();
    let dense_chunks = cache_chunks.into_values().flatten().collect::<Vec<_>>();
    let dense_vectors = Vec::new();
    let dense_query = Vec::new();
    let dense = top_k_dense(&dense_query, &dense_chunks, &dense_vectors, 20);
    let empty_sparse: Vec<SearchHit> = Vec::new();
    let fusion = fuse_continuous_convex(
        &empty_sparse,
        &dense,
        1.0,
        NormalizationStrategy::CandidateMinMax,
    );
    let fusion = fusion.truncate(limit);

    // Dense top-k + fusion.
    let dense = top_k_dense(&dense_query, &dense_chunks, &dense_vectors, 20);
    let empty_sparse: Vec<SearchHit> = Vec::new();
    let fusion = fuse_continuous_convex(
        &empty_sparse,
        &dense,
        1.0,
        NormalizationStrategy::CandidateMinMax,
    );
    fusion.truncate(limit);

    // Graph expansion + synthesis prompt.
    let manifest = scan_corpus_manifest(repo_root)?;
    let graph = KnowledgeGraph::build_from_manifest(repo_root, &manifest)?;
    let chunks_by_path = cache.chunks_by_path();
    let expanded = expand_hits_with_graph(&fusion, &graph, &chunks_by_path, 2);

    let prompt = SynthesisPromptBuilder::build(question, &expanded);
    let resolved_endpoint = endpoint
        .or_else(|| SynthesisPromptBuilder::resolve_local_inference_endpoint())
        .ok_or(ServiceError::NoInferenceEndpoint)?;

    Ok(AskResult {
        endpoint: resolved_endpoint,
        prompt,
        sources: prompt.sources.clone(),
        dense_chunks,
        dense_vectors,
        dense_query,
        fusion: expanded,
    })
}

// ---------------------------------------------------------------------------
// Surface / facts
// ---------------------------------------------------------------------------

pub struct SurfaceVerifyResult {
    pub ledger: EpistemicFactLedger,
    pub claims_checks: Vec<ClaimCheckOutcome>,
}

pub fn harvest_surface_ledger(root: &Path) -> Result<EpistemicFactLedger, ServiceError> {
    harvest_live_ledger(root, None).map_err(ServiceError::Io)
}

pub fn surface_verify(
    ledger: &EpistemicFactLedger,
    root: &Path,
    verify: bool,
) -> Result<SurfaceVerifyResult, ServiceError> {
    if !verify {
        return Ok(SurfaceVerifyResult {
            ledger: ledger.clone(),
            claims_checks: Vec::new(),
        });
    }

    let verdicts = verify_event_proofs(&ledger.events, git_proof_resolver(root));
    let checks = check_claims(&ledger.claims, shell_claim_runner(root));

    Ok(SurfaceVerifyResult {
        ledger: ledger.clone(),
        claims_checks: checks,
    })
}

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

pub fn render_public_surface(ledger: &EpistemicFactLedger) -> (String, String) {
    (
        render_public_surface_markdown(ledger),
        render_public_surface_html(ledger),
    )
}

pub fn check_published_surface_freshness(path: &Path, hours: u64) -> Result<u64, ServiceError> {
    let raw = std::fs::read_to_string(path).map_err(|e| ServiceError::Io(e))?;
    let generated_at = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("generated_at").and_then(|g| g.as_str().map(str::to_string)))
        .ok_or_else(|| ServiceError::InvalidInput(format!("{} has no readable generated_at", path.display())))?;
    let now = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .map_err(|e| ServiceError::Io(e))?;
    check_freshness(&generated_at, &now, hours.saturating_mul(3600))
        .map_err(|e| ServiceError::InvalidInput(e.to_string()))
}


