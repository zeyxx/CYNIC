use std::fs;
use std::path::PathBuf;

use cynic_knowledge::{
    AuthorityTier, KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph, SearchHit,
    expand_hits_with_graph, sync_workspace_corpus,
};

fn test_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cynic_graph_rag_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_graph_rag_expansion_enriches_retrieval_with_parent_contracts() {
    let dir = test_temp_dir("expansion");
    let root = &dir;

    // 1. Create docs/specs/network.md
    let specs_dir = root.join("docs/specs");
    fs::create_dir_all(&specs_dir).unwrap();
    let spec_path = specs_dir.join("network.md");
    fs::write(
        &spec_path,
        "# Network Specification\nWireGuard mesh topology.\nGoverned by [foundation contract](../../contracts/foundation.json).\n",
    ).unwrap();

    // 2. Create contracts/foundation.json
    let contracts_dir = root.join("contracts");
    fs::create_dir_all(&contracts_dir).unwrap();
    let contract_path = contracts_dir.join("foundation.json");
    fs::write(
        &contract_path,
        "{\n  \"schema\": \"cynic.foundation.v1\",\n  \"purpose\": \"Sovereign foundation invariants.\"\n}\n",
    ).unwrap();

    // 3. Sync corpus
    let cache_file = root.join("cache.json");
    let (cache, index) = sync_workspace_corpus(root, Some(&cache_file), 0);

    // Initial BM25 hit matches network spec
    let initial_hits = index.search("WireGuard mesh", 1);
    assert_eq!(initial_hits.len(), 1);
    assert_eq!(initial_hits[0].chunk.source_path, "docs/specs/network.md");

    // Build KnowledgeGraph with explicit edge
    let mut graph = KnowledgeGraph::default();
    graph.nodes.insert("docs/specs/network.md".into());
    graph.nodes.insert("contracts/foundation.json".into());
    graph.edges.push(KnowledgeEdge {
        source: "docs/specs/network.md".into(),
        target: "contracts/foundation.json".into(),
        kind: KnowledgeEdgeKind::CitesDocument,
        label: Some("governed_by".into()),
    });

    // 4. Perform Graph-RAG expansion
    let chunks_by_path = cache.chunks_by_path();
    let expanded_hits = expand_hits_with_graph(&initial_hits, &graph, &chunks_by_path, 2);

    // Assert that the parent contract was pulled into the context
    assert_eq!(expanded_hits.len(), 2);
    assert_eq!(expanded_hits[0].chunk.source_path, "docs/specs/network.md");
    assert_eq!(
        expanded_hits[1].chunk.source_path,
        "contracts/foundation.json"
    );
    assert_eq!(
        expanded_hits[1].chunk.authority,
        AuthorityTier::L0Constitution
    );
    assert!(
        expanded_hits[1]
            .chunk
            .content
            .contains("Sovereign foundation invariants.")
    );
}
