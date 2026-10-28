//! Native synchronization of workspace knowledge graph and documents into SurrealDB.

use serde::Serialize;
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
    pub batches_executed: usize,
    pub elapsed_ms: u64,
}

pub fn execute_storage_sync(
    client: &SurrealHttpClient,
    endpoint: &str,
) -> Result<StorageSyncReport, String> {
    let start = Instant::now();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let repo_root = crate::find_repo_root(&cwd).unwrap_or(cwd);

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
        let escaped = path.replace('"', "\\\"");
        statements.push(format!(
            "UPSERT doc:⟨{escaped}⟩ SET path = \"{escaped}\", kind = \"{kind}\";"
        ));
    }

    // 2. Edges
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

    // 3. Batch execution (batches of 100 statements)
    let batch_size = 100;
    let mut batches_executed = 0;
    for chunk in statements.chunks(batch_size) {
        let batch_sql = chunk.join("\n");
        client.execute_sql(&batch_sql)?;
        batches_executed += 1;
    }

    Ok(StorageSyncReport {
        schema: "cynic.storage.sync_report.v1",
        status: "synced".to_string(),
        endpoint: endpoint.to_string(),
        documents_synced: graph.nodes.len(),
        edges_synced: graph.edges.len(),
        batches_executed,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}
