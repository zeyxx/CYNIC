use std::collections::{HashMap, HashSet};

use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use crate::domain::graph::KnowledgeGraph;
use crate::domain::scorer::SearchHit;

/// Expands a list of initial search hits by traversing the knowledge graph
/// to retrieve relevant parent contracts and authoritative dependencies.
pub fn expand_hits_with_graph(
    hits: &[SearchHit],
    graph: &KnowledgeGraph,
    all_chunks_by_path: &HashMap<String, Vec<DocumentChunk>>,
    max_expansion: usize,
) -> Vec<SearchHit> {
    let mut expanded = hits.to_vec();
    if max_expansion == 0 || hits.is_empty() {
        return expanded;
    }

    let mut already_included_paths: HashSet<String> =
        hits.iter().map(|h| h.chunk.source_path.clone()).collect();

    let mut candidate_targets = Vec::new();

    // 1. Traverse outgoing and incoming edges for each retrieved document/code
    for hit in hits {
        let path = &hit.chunk.source_path;

        // Check outgoing references (e.g. doc cites a contract)
        for edge in graph.outgoing_edges(path) {
            if !already_included_paths.contains(&edge.target) {
                candidate_targets.push((edge.target.clone(), &edge.kind));
            }
        }

        // Check incoming references (e.g. contract enforces this code/test)
        for edge in graph.incoming_edges(path) {
            if !already_included_paths.contains(&edge.source) {
                candidate_targets.push((edge.source.clone(), &edge.kind));
            }
        }
    }

    // 2. Sort candidate targets by authority (L0 Constitution first, then L1)
    candidate_targets.sort_by(|(path_a, _), (path_b, _)| {
        let auth_a = AuthorityTier::from_path(path_a);
        let auth_b = AuthorityTier::from_path(path_b);
        auth_b.cmp(&auth_a) // Higher authority first
    });

    // 3. Select top candidates and inject representative chunks
    let mut added_count = 0;
    for (target_path, _kind) in candidate_targets {
        if added_count >= max_expansion {
            break;
        }

        if already_included_paths.insert(target_path.clone()) {
            if let Some(chunks) = all_chunks_by_path.get(&target_path) {
                // Pick the first chunk (overview / title section) of the linked document/contract
                if let Some(first_chunk) = chunks.first() {
                    let mut expanded_chunk = first_chunk.clone();
                    expanded_chunk
                        .section_hierarchy
                        .insert(0, "Graph-Linked Context".to_string());

                    expanded.push(SearchHit {
                        chunk: expanded_chunk,
                        score: 1.0, // Base baseline score for graph context
                    });
                    added_count += 1;
                }
            }
        }
    }

    expanded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::graph::{KnowledgeEdge, KnowledgeEdgeKind};

    #[test]
    fn test_graph_expansion_pulls_parent_contracts() {
        let mut graph = KnowledgeGraph::default();
        graph.nodes.insert("docs/specs/test_spec.md".into());
        graph.nodes.insert("contracts/foundation.json".into());

        graph.edges.push(KnowledgeEdge {
            source: "docs/specs/test_spec.md".into(),
            target: "contracts/foundation.json".into(),
            kind: KnowledgeEdgeKind::CitesDocument,
            label: Some("governed_by".into()),
        });

        let mut chunks_by_path = HashMap::new();
        let spec_chunk = DocumentChunk::new(
            "docs/specs/test_spec.md".into(),
            1,
            10,
            vec!["Spec".into()],
            "Spec content".into(),
            AuthorityTier::L1Specification,
        );
        let foundation_chunk = DocumentChunk::new(
            "contracts/foundation.json".into(),
            1,
            20,
            vec!["Contract".into()],
            "Constitution foundation".into(),
            AuthorityTier::L0Constitution,
        );

        chunks_by_path.insert("docs/specs/test_spec.md".into(), vec![spec_chunk.clone()]);
        chunks_by_path.insert("contracts/foundation.json".into(), vec![foundation_chunk]);

        let initial_hits = vec![SearchHit {
            chunk: spec_chunk,
            score: 5.0,
        }];

        let expanded = expand_hits_with_graph(&initial_hits, &graph, &chunks_by_path, 2);

        // Should contain both initial spec and expanded contract
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0].chunk.source_path, "docs/specs/test_spec.md");
        assert_eq!(expanded[1].chunk.source_path, "contracts/foundation.json");
        assert_eq!(expanded[1].chunk.authority, AuthorityTier::L0Constitution);
        assert_eq!(
            expanded[1].chunk.section_hierarchy[0],
            "Graph-Linked Context"
        );
    }
}
