use std::fs;
use std::path::PathBuf;

use cynic_knowledge::{
    AuthorityTier, KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph, SearchHit,
    expand_hits_with_graph, expand_hits_with_topological_contexts, sync_workspace_corpus,
};
use cynic_surreal::domain::topology::TopologicalContext;
use std::collections::HashMap;

fn test_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cynic_topo_gain_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_topological_expansion_recall_gain() {
    let dir = test_temp_dir("gain");
    let root = &dir;

    // Create a small corpus with a spec and a contract
    let specs_dir = root.join("docs/specs");
    fs::create_dir_all(&specs_dir).unwrap();
    let spec_path = specs_dir.join("network.md");
    fs::write(
        &spec_path,
        "# Network Specification\nWireGuard mesh topology. Governed by foundation contract.\n",
    )
    .unwrap();

    let contracts_dir = root.join("contracts");
    fs::create_dir_all(&contracts_dir).unwrap();
    let contract_path = contracts_dir.join("foundation.json");
    fs::write(
        &contract_path,
        "{\n  \"schema\": \"cynic.foundation.v1\",\n  \"purpose\": \"Sovereign foundation invariants.\"\n}\n",
    )
    .unwrap();

    let cache_file = root.join("cache.json");
    let (cache, index) = sync_workspace_corpus(root, Some(&cache_file), 0);

    let initial_hits = index.search("WireGuard mesh", 1);
    assert_eq!(initial_hits.len(), 1);
    assert_eq!(initial_hits[0].chunk.source_path, "docs/specs/network.md");

    let chunks_by_path = cache.chunks_by_path();

    // Baseline: graph expansion with explicit edge
    let mut graph = KnowledgeGraph::default();
    graph.nodes.insert("docs/specs/network.md".into());
    graph.nodes.insert("contracts/foundation.json".into());
    graph.edges.push(KnowledgeEdge {
        source: "docs/specs/network.md".into(),
        target: "contracts/foundation.json".into(),
        kind: KnowledgeEdgeKind::CitesDocument,
        label: Some("governed_by".into()),
    });

    let graph_expanded = expand_hits_with_graph(&initial_hits, &graph, &chunks_by_path, 2);
    let graph_paths: Vec<&str> = graph_expanded
        .iter()
        .map(|h| h.chunk.source_path.as_str())
        .collect();

    // Topology expansion: uses SurrealDB TopologicalContext
    let a_chunk = initial_hits.first().unwrap();
    let chunk_id = a_chunk.chunk.chunk_id.clone();
    let mut ctx_map = HashMap::new();
    ctx_map.insert(
        chunk_id,
        TopologicalContext {
            parent_path: String::new(),
            parent_authority: String::new(),
            enforces: vec![],
            depends_on: vec![],
            cites: vec!["contracts/foundation.json".into()],
            enforced_by: vec![],
            depended_on_by: vec![],
        },
    );
    let lex_hits = vec![cynic_knowledge::LexicalHit {
        chunk: a_chunk.chunk,
        score: a_chunk.score,
    }];
    let topo_expanded = expand_hits_with_topological_contexts(&lex_hits, &ctx_map, &chunks_by_path, "", 2);
    let topo_paths: Vec<&str> = topo_expanded
        .iter()
        .map(|h| h.chunk.source_path.as_str())
        .collect();

    // Both should include the contract
    assert!(graph_paths.contains(&"contracts/foundation.json"));
    assert!(topo_paths.contains(&"contracts/foundation.json"));

    // Recall comparison: count unique source paths retrieved
    let graph_recall = graph_paths.len();
    let topo_recall = topo_paths.len();

    eprintln!("Graph recall: {} paths", graph_recall);
    eprintln!("Topology recall: {} paths", topo_recall);

    // Topology should be at least as good as graph
    assert!(topo_recall >= graph_recall);
}
