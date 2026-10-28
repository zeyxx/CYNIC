//! Native synchronization of workspace knowledge graph, documents, and chunks into SurrealDB.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cynic_knowledge::domain::graph::{KnowledgeEdgeKind, KnowledgeGraph};
use cynic_storage::SurrealHttpClient;
use cynic_storage::SurrealStoragePort;

#[derive(Debug, Serialize)]
pub struct StorageSyncReport {
    pub schema: &'static str,
    pub status: String,
    pub endpoint: String,
    pub documents_synced: usize,
    pub edges_synced: usize,
    pub chunks_synced: usize,
    pub contains_edges_synced: usize,
    pub embeddings_computed: usize,
    pub batches_executed: usize,
    pub elapsed_ms: u64,
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() || current.join("Cargo.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

pub fn execute_storage_sync(
    client: &SurrealHttpClient,
    endpoint: &str,
    embed: bool,
) -> Result<StorageSyncReport, String> {
    let start = Instant::now();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let repo_root = find_repo_root(&cwd).unwrap_or(cwd);

    let manifest = cynic_knowledge::scan_corpus_manifest(&repo_root);
    let graph = KnowledgeGraph::build_from_manifest(&repo_root, &manifest);

    let mut statements = Vec::new();

    // 1. Documents
    for path in &graph.nodes {
        let kind = if path.starts_with("contracts/") {
            "contract"
        } else if path.starts_with("crates/") {
            "crate"
        } else if path.contains("specs/") {
            "spec"
        } else {
            "doc"
        };
        let escaped_id = path.replace('"', "\\\"");
        let path_json = serde_json::to_string(path).unwrap_or_default();
        let kind_json = serde_json::to_string(kind).unwrap_or_default();
        statements.push(format!(
            "UPSERT doc:⟨{escaped_id}⟩ SET path = {path_json}, kind = {kind_json};"
        ));
    }

    // 2. Knowledge Graph Edges
    for edge in &graph.edges {
        let src = edge.source.replace('"', "\\\"");
        let tgt = edge.target.replace('"', "\\\"");
        let rel = match edge.kind {
            KnowledgeEdgeKind::CitesDocument => "cites",
            KnowledgeEdgeKind::EnforcesContract => "enforces",
            KnowledgeEdgeKind::CrateDependency => "depends_on",
            KnowledgeEdgeKind::ParentSection => "parent_of",
        };
        statements.push(format!("RELATE doc:⟨{src}⟩ -> {rel} -> doc:⟨{tgt}⟩;"));
    }

    // 3. Document Chunks & `contains_chunk` Graph Edges
    let index = cynic_knowledge::index_workspace(&repo_root);
    let chunks = index.chunks();

    // 3a. Optional Dense Embeddings via Cluster/Registry
    let mut embeddings: Option<Vec<Vec<f32>>> = None;
    let mut embeddings_computed = 0;

    if embed && !chunks.is_empty() {
        let backend = cynic_inference::adapters::embedding::http::EmbeddingBackend::from_env()
            .map_err(|e| format!("failed to initialize embedding backend: {e}"))?;
        let bridge = cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge::probe(
            backend,
            "cynic storage sync probe",
        )
        .map_err(|e| format!("embedding backend probe failed: {e}"))?;

        use cynic_knowledge::ChunkEmbedder;
        let vectors = bridge
            .embed_chunks(chunks)
            .map_err(|e| format!("chunk embedding failed: {e}"))?;
        embeddings_computed = vectors.len();
        embeddings = Some(vectors);
    }

    for (idx, chunk) in chunks.iter().enumerate() {
        let chunk_id_esc = chunk.chunk_id.replace('"', "\\\"");
        let source_path_esc = chunk.source_path.replace('"', "\\\"");

        let chunk_id_json = serde_json::to_string(&chunk.chunk_id).unwrap_or_default();
        let source_path_json = serde_json::to_string(&chunk.source_path).unwrap_or_default();
        let content_json = serde_json::to_string(&chunk.content).unwrap_or_default();
        let section_json =
            serde_json::to_string(&chunk.section_hierarchy.join(" > ")).unwrap_or_default();
        let authority_str = format!("{:?}", chunk.authority);
        let authority_json = serde_json::to_string(&authority_str).unwrap_or_default();

        let mut set_clauses = format!(
            "chunk_id = {chunk_id_json}, source_path = {source_path_json}, line_start = {}, line_end = {}, section = {section_json}, authority = {authority_json}, content = {content_json}",
            chunk.line_start, chunk.line_end
        );

        if let Some(ref vecs) = embeddings {
            if let Some(vec) = vecs.get(idx) {
                let vec_json = serde_json::to_string(vec).unwrap_or_default();
                set_clauses.push_str(&format!(", embedding = {vec_json}"));
            }
        }

        statements.push(format!("UPSERT chunk:⟨{chunk_id_esc}⟩ SET {set_clauses};"));
        statements.push(format!(
            "RELATE doc:⟨{source_path_esc}⟩ -> contains_chunk -> chunk:⟨{chunk_id_esc}⟩;"
        ));
    }

    // 4. Batch execution (batches of 25 statements for resilient payloads)
    let batch_size = 25;
    let mut batches_executed = 0;
    for batch in statements.chunks(batch_size) {
        let batch_sql = batch.join("\n");
        client.execute_sql(&batch_sql)?;
        batches_executed += 1;
    }

    Ok(StorageSyncReport {
        schema: "cynic.storage.sync_report.v1",
        status: "synced".to_string(),
        endpoint: endpoint.to_string(),
        documents_synced: graph.nodes.len(),
        edges_synced: graph.edges.len(),
        chunks_synced: chunks.len(),
        contains_edges_synced: chunks.len(),
        embeddings_computed,
        batches_executed,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}
