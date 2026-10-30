use std::collections::{HashMap, HashSet};

use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use crate::domain::graph::{CorpusEdgeKind, CorpusGraph, KnowledgeGraph};
use crate::domain::scorer::{LexicalHit, SearchHit};
use cynic_surreal::domain::topology::TopologicalContext;

/// Default number of graph hops performed during expansion.
pub const DEFAULT_MAX_HOPS: usize = 2;
/// Default per-hop score decay factor.
pub const DEFAULT_DECAY_FACTOR: f64 = 0.7;

/// Edge weight applied according to the semantic kind of a corpus edge.
pub fn edge_weight(kind: &CorpusEdgeKind) -> f64 {
    match kind {
        CorpusEdgeKind::EnforcesContract => 1.2,
        CorpusEdgeKind::CitesDocument => 1.0,
        CorpusEdgeKind::CrateDependency => 0.9,
        CorpusEdgeKind::ParentSection => 0.8,
    }
}

/// Weight applied to a topological (SurrealDB) relation.
pub fn topological_weight(label: &str) -> f64 {
    match label {
        "enforces" | "enforced_by" => 1.2,
        "cites" => 1.0,
        "depends_on" | "depended_on_by" => 0.9,
        "parent" => 0.8,
        _ => 1.0,
    }
}

/// Expands a list of initial search hits by traversing the knowledge graph
/// to retrieve relevant parent contracts and authoritative dependencies,
/// selecting the most relevant chunk in each linked document for the given query.
///
/// Performs multi-hop traversal (up to `DEFAULT_MAX_HOPS`) with per-hop score
/// decay (`DEFAULT_DECAY_FACTOR`) and cycle detection via a visited node set.
pub fn expand_hits_with_graph_query<'a>(
    hits: &[SearchHit<'a>],
    graph: &KnowledgeGraph,
    all_chunks_by_path: &'a HashMap<String, Vec<DocumentChunk>>,
    query: &str,
    max_expansion: usize,
) -> Vec<SearchHit<'a>> {
    expand_hits_with_graph_query_hops(
        hits,
        graph,
        all_chunks_by_path,
        query,
        max_expansion,
        DEFAULT_MAX_HOPS,
        DEFAULT_DECAY_FACTOR,
    )
}

/// Variant allowing explicit configuration of `max_hops` and `decay`.
pub fn expand_hits_with_graph_query_hops<'a>(
    hits: &[SearchHit<'a>],
    graph: &KnowledgeGraph,
    all_chunks_by_path: &'a HashMap<String, Vec<DocumentChunk>>,
    query: &str,
    max_expansion: usize,
    max_hops: usize,
    decay: f64,
) -> Vec<SearchHit<'a>> {
    let mut expanded: Vec<SearchHit<'a>> = hits.to_vec();
    if max_expansion == 0 || hits.is_empty() || max_hops == 0 {
        return expanded;
    }

    let mut already_included_paths: HashSet<String> =
        hits.iter().map(|h| h.chunk.source_path.clone()).collect();

    // Visited set for strict cycle prevention.
    let mut visited: HashSet<String> = already_included_paths.clone();

    // (node_path, propagated_score)
    let mut current_level: Vec<(String, f64)> =
        hits.iter().map(|h| (h.chunk.source_path.clone(), h.score)).collect();

    // Collect candidate target paths with their propagated (decayed) scores.
    let mut candidates: Vec<(String, f64)> = Vec::new();

    for _hop in 1..=max_hops {
        let mut next_level: Vec<(String, f64)> = Vec::new();
        for (path, parent_score) in &current_level {
            // Outgoing edges.
            for edge in graph.outgoing_edges(path) {
                if visited.contains(&edge.target) {
                    continue;
                }
                let w = edge_weight(&edge.kind);
                let score = parent_score * w * decay;
                visited.insert(edge.target.clone());
                candidates.push((edge.target.clone(), score));
                next_level.push((edge.target.clone(), score));
            }
            // Incoming edges.
            for edge in graph.incoming_edges(path) {
                if visited.contains(&edge.source) {
                    continue;
                }
                let w = edge_weight(&edge.kind);
                let score = parent_score * w * decay;
                visited.insert(edge.source.clone());
                candidates.push((edge.source.clone(), score));
                next_level.push((edge.source.clone(), score));
            }
        }
        current_level = next_level;
        if current_level.is_empty() {
            break;
        }
    }

    // Sort candidate targets by authority (L0 Constitution first, then L1),
    // then by propagated score descending.
    candidates.sort_by(|(path_a, score_a), (path_b, score_b)| {
        let auth_a = AuthorityTier::from_path(path_a);
        let auth_b = AuthorityTier::from_path(path_b);
        auth_b
            .cmp(&auth_a)
            .then_with(|| score_b.partial_cmp(score_a).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Extract query terms for relevance matching (lowercase alphanumeric, len >= 3)
    let query_terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_lowercase())
        .collect();

    // Select top candidates and inject representative chunks.
    let mut added_count = 0;
    for (target_path, score) in candidates {
        if added_count >= max_expansion {
            break;
        }

        if !already_included_paths.insert(target_path.clone()) {
            continue;
        }

        if let Some(chunks) = all_chunks_by_path.get(&target_path) {
            if chunks.is_empty() {
                continue;
            }

            let best_chunk = if !query_terms.is_empty() {
                let mut best_c = chunks.first().unwrap();
                let mut max_score = 0usize;

                for chunk in chunks {
                    let content_lower = chunk.content.to_lowercase();
                    let mut chunk_score = 0usize;

                    for term in &query_terms {
                        if content_lower.contains(term) {
                            chunk_score += 1;
                        }
                        for h in &chunk.section_hierarchy {
                            if h.to_lowercase().contains(term) {
                                chunk_score += 2;
                            }
                        }
                    }

                    if chunk_score > max_score {
                        max_score = chunk_score;
                        best_c = chunk;
                    }
                }

                best_c
            } else {
                chunks.first().unwrap()
            };

            expanded.push(SearchHit {
                chunk: best_chunk,
                score,
            });
            added_count += 1;
        }
    }

    expanded
}

/// Backward-compatible alias using `CorpusGraph` (KnowledgeGraph alias).
pub fn expand_hits_with_graph<'a>(
    hits: &[SearchHit<'a>],
    graph: &KnowledgeGraph,
    all_chunks_by_path: &'a HashMap<String, Vec<DocumentChunk>>,
    max_expansion: usize,
) -> Vec<SearchHit<'a>> {
    expand_hits_with_graph_query(hits, graph, all_chunks_by_path, "", max_expansion)
}

/// Expands hits using SurrealDB `TopologicalContext` records.
pub fn expand_hits_with_topological_contexts<'a>(
    hits: &[LexicalHit<'a>],
    contexts: &HashMap<String, TopologicalContext>,
    all_chunks_by_path: &'a HashMap<String, Vec<DocumentChunk>>,
    query: &str,
    max_expansion: usize,
) -> Vec<LexicalHit<'a>> {
    let decay = DEFAULT_DECAY_FACTOR;
    let mut expanded: Vec<LexicalHit<'a>> = hits.to_vec();
    if max_expansion == 0 || hits.is_empty() {
        return expanded;
    }

    let mut already_included_paths: HashSet<String> =
        hits.iter().map(|h| h.chunk.source_path.clone()).collect();

    // Collect (path, weight, parent_score) candidates.
    let mut candidates: Vec<(String, f64, f64)> = Vec::new(); // path, score, weight unused
    for hit in hits {
        // Resolve context by chunk id, falling back to parent_path match.
        let ctx = contexts
            .get(&hit.chunk.chunk_id)
            .or_else(|| {
                contexts
                    .values()
                    .find(|c| !c.parent_path.is_empty() && c.parent_path == hit.chunk.source_path)
            });
        if let Some(ctx) = ctx {
            let mut neighbors: Vec<(&str, &str)> = Vec::new();
            for p in &ctx.enforces {
                neighbors.push((p.as_str(), "enforces"));
            }
            for p in &ctx.enforced_by {
                neighbors.push((p.as_str(), "enforced_by"));
            }
            for p in &ctx.depends_on {
                neighbors.push((p.as_str(), "depends_on"));
            }
            for p in &ctx.depended_on_by {
                neighbors.push((p.as_str(), "depended_on_by"));
            }
            for p in &ctx.cites {
                neighbors.push((p.as_str(), "cites"));
            }
            if !ctx.parent_path.is_empty() {
                neighbors.push((ctx.parent_path.as_str(), "parent"));
            }

            for (target, label) in neighbors {
                if already_included_paths.contains(target) {
                    continue;
                }
                let w = topological_weight(label);
                let score = hit.score * w * decay;
                candidates.push((target.to_string(), score, w));
            }
        }
    }

    // Sort by authority then score.
    candidates.sort_by(|(pa, sa, _), (pb, sb, _)| {
        let auth_a = AuthorityTier::from_path(pa);
        let auth_b = AuthorityTier::from_path(pb);
        auth_b
            .cmp(&auth_a)
            .then_with(|| sb.partial_cmp(sa).unwrap_or(std::cmp::Ordering::Equal))
    });

    let query_terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_lowercase())
        .collect();

    let mut added_count = 0;
    for (target_path, score, _) in candidates {
        if added_count >= max_expansion {
            break;
        }
        if !already_included_paths.insert(target_path.clone()) {
            continue;
        }
        if let Some(chunks) = all_chunks_by_path.get(&target_path) {
            if chunks.is_empty() {
                continue;
            }
            let best_chunk = if !query_terms.is_empty() {
                let mut best_c = chunks.first().unwrap();
                let mut max_score = 0usize;
                for chunk in chunks {
                    let content_lower = chunk.content.to_lowercase();
                    let mut chunk_score = 0usize;
                    for term in &query_terms {
                        if content_lower.contains(term) {
                            chunk_score += 1;
                        }
                        for h in &chunk.section_hierarchy {
                            if h.to_lowercase().contains(term) {
                                chunk_score += 2;
                            }
                        }
                    }
                    if chunk_score > max_score {
                        max_score = chunk_score;
                        best_c = chunk;
                    }
                }
                best_c
            } else {
                chunks.first().unwrap()
            };

            expanded.push(LexicalHit {
                chunk: best_chunk,
                score,
            });
            added_count += 1;
        }
    }

    expanded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::graph::{KnowledgeEdge, KnowledgeEdgeKind};

    fn make_chunks() -> HashMap<String, Vec<DocumentChunk>> {
        let mut m = HashMap::new();
        let a = DocumentChunk::new("A".into(), 1, 1, vec![], "A content".into(), AuthorityTier::L3Operation);
        let b = DocumentChunk::new("B".into(), 1, 1, vec![], "B content".into(), AuthorityTier::L2Design);
        let c = DocumentChunk::new("C".into(), 1, 1, vec![], "C content".into(), AuthorityTier::L1Specification);
        m.insert("A".into(), vec![a]);
        m.insert("B".into(), vec![b]);
        m.insert("C".into(), vec![c]);
        m
    }

    #[test]
    fn test_multihop_traversal() {
        let mut graph = KnowledgeGraph::default();
        graph.nodes.insert("A".into());
        graph.nodes.insert("B".into());
        graph.nodes.insert("C".into());
        graph.edges.push(KnowledgeEdge { source: "A".into(), target: "B".into(), kind: KnowledgeEdgeKind::CitesDocument, label: None });
        graph.edges.push(KnowledgeEdge { source: "B".into(), target: "C".into(), kind: KnowledgeEdgeKind::CitesDocument, label: None });

        let chunks = make_chunks();
        let a_chunk = chunks.get("A").unwrap().first().unwrap();
        let hits = vec![SearchHit { chunk: a_chunk, score: 1.0 }];
        let expanded = expand_hits_with_graph(&hits, &graph, &chunks, 10);
        let paths: Vec<&str> = expanded.iter().map(|h| h.chunk.source_path.as_str()).collect();
        assert!(paths.contains(&"A"));
        assert!(paths.contains(&"B"));
        assert!(paths.contains(&"C"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = KnowledgeGraph::default();
        graph.nodes.insert("A".into());
        graph.nodes.insert("B".into());
        graph.edges.push(KnowledgeEdge { source: "A".into(), target: "B".into(), kind: KnowledgeEdgeKind::CitesDocument, label: None });
        graph.edges.push(KnowledgeEdge { source: "B".into(), target: "A".into(), kind: KnowledgeEdgeKind::CitesDocument, label: None });

        let chunks = make_chunks();
        let a_chunk = chunks.get("A").unwrap().first().unwrap();
        let hits = vec![SearchHit { chunk: a_chunk, score: 1.0 }];
        let expanded = expand_hits_with_graph(&hits, &graph, &chunks, 10);
        // Should not duplicate A
        let count_a = expanded.iter().filter(|h| h.chunk.source_path == "A").count();
        assert_eq!(count_a, 1);
    }

    #[test]
    fn test_score_decay() {
        let mut graph = KnowledgeGraph::default();
        graph.nodes.insert("A".into());
        graph.nodes.insert("B".into());
        graph.nodes.insert("C".into());
        graph.edges.push(KnowledgeEdge { source: "A".into(), target: "B".into(), kind: KnowledgeEdgeKind::CitesDocument, label: None });
        graph.edges.push(KnowledgeEdge { source: "B".into(), target: "C".into(), kind: KnowledgeEdgeKind::CitesDocument, label: None });

        let chunks = make_chunks();
        let a_chunk = chunks.get("A").unwrap().first().unwrap();
        let hits = vec![SearchHit { chunk: a_chunk, score: 1.0 }];
        let expanded = expand_hits_with_graph(&hits, &graph, &chunks, 10);
        let b = expanded.iter().find(|h| h.chunk.source_path == "B").unwrap();
        let c = expanded.iter().find(|h| h.chunk.source_path == "C").unwrap();
        assert!((b.score - 0.7).abs() < 1e-9);
        assert!((c.score - 0.49).abs() < 1e-9);
    }

    #[test]
    fn test_topological_contexts() {
        use cynic_surreal::domain::topology::TopologicalContext;
        let mut chunks = make_chunks();
        chunks.insert(
            "P".into(),
            vec![DocumentChunk::new(
                "P".into(),
                1,
                1,
                vec![],
                "P".into(),
                AuthorityTier::L0Constitution,
            )],
        );

        let a_chunk = chunks.get("A").unwrap().first().unwrap();
        let chunk_id = a_chunk.chunk_id.clone();
        let hits = vec![LexicalHit {
            chunk: a_chunk,
            score: 1.0,
        }];

        let mut ctx_map = HashMap::new();
        ctx_map.insert(
            chunk_id,
            TopologicalContext {
                parent_path: "P".into(),
                parent_authority: "L0".into(),
                enforces: vec![],
                depends_on: vec![],
                cites: vec!["B".into()],
                enforced_by: vec![],
                depended_on_by: vec![],
            },
        );

        let expanded = expand_hits_with_topological_contexts(&hits, &ctx_map, &chunks, "", 10);
        let paths: Vec<&str> = expanded.iter().map(|h| h.chunk.source_path.as_str()).collect();
        assert!(paths.contains(&"B"));
    }
}
