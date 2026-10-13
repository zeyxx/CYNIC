use cynic_knowledge::{KnowledgeEdgeKind, KnowledgeGraph, scan_corpus_manifest};
use std::path::Path;

#[test]
fn knowledge_graph_has_zero_broken_references() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();

    let manifest = scan_corpus_manifest(repo_root);
    let graph = KnowledgeGraph::build_from_manifest(repo_root, &manifest);

    // Fail-Closed: Zero broken references allowed in documentation
    assert!(
        graph.broken_references.is_empty(),
        "Broken references found in corpus: {:?}",
        graph.broken_references
    );

    // Graph must have nodes and edges
    assert!(
        graph.nodes.len() >= 150,
        "Expected at least 150 nodes in knowledge graph"
    );
    assert!(!graph.edges.is_empty(), "Graph must contain edges");

    // Must have cross-document citation edges
    let doc_citations = graph
        .edges
        .iter()
        .filter(|e| e.kind == KnowledgeEdgeKind::CitesDocument)
        .count();
    assert!(
        doc_citations > 20,
        "Expected at least 20 cross-document citations, found {}",
        doc_citations
    );

    // Must have contract enforcement edges
    let contract_enforcements = graph
        .edges
        .iter()
        .filter(|e| e.kind == KnowledgeEdgeKind::EnforcesContract)
        .count();
    assert!(
        contract_enforcements >= 3,
        "Expected at least 3 contract enforcement edges"
    );
}
