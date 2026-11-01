//! Topological retrieval gain benchmark: BM25 vs BM25 + topological expansion.
//!
//! Reads the graded labels in `retrieval_gain_labels.json`, runs BM25-only
//! retrieval and BM25 + topological expansion, reports MRR + nDCG@5 per query.

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_knowledge::{
    KnowledgeGraph, expand_hits_with_topological_contexts, scan_corpus_manifest, sync_workspace_corpus,
};
use cynic_surreal::domain::topology::TopologicalContext;
use serde::Deserialize;

const TOP_K: usize = 5;

#[derive(Debug, Deserialize)]
struct Fixture {
    schema: String,
    queries: Vec<LabeledQuery>,
}

#[derive(Debug, Deserialize)]
struct LabeledQuery {
    id: String,
    query: String,
    trap: bool,
    grades: HashMap<String, u32>,
}

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find repo root containing contracts/");
        }
    }
    dir
}

fn grade_hit(path: &str, grades: &HashMap<String, u32>) -> u32 {
    grades
        .iter()
        .filter(|(labeled, _)| path.contains(labeled.as_str()))
        .map(|(_, grade)| *grade)
        .max()
        .unwrap_or(0)
}

fn ndcg_at_k(grades_ranked: &[u32]) -> f64 {
    let dcg: f64 = grades_ranked
        .iter()
        .enumerate()
        .map(|(i, grade)| (f64::from(1u32 << grade) - 1.0) / ((i + 2) as f64).log2())
        .sum();
    let mut ideal = grades_ranked.to_vec();
    ideal.sort_unstable_by(|a, b| b.cmp(a));
    let idcg: f64 = ideal
        .iter()
        .enumerate()
        .map(|(i, grade)| (f64::from(1u32 << grade) - 1.0) / ((i + 2) as f64).log2())
        .sum();
    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

fn build_topological_contexts(
    graph: &KnowledgeGraph,
    _chunks: &cynic_knowledge::LexicalIndex,
) -> HashMap<String, TopologicalContext> {
    let mut ctx_map = HashMap::new();
    for node in &graph.nodes {
        let mut enforces = Vec::new();
        let mut depends_on = Vec::new();
        let mut cites = Vec::new();
        let mut enforced_by = Vec::new();
        let mut depended_on_by = Vec::new();
        for edge in &graph.edges {
            if edge.source == *node {
                match edge.kind {
                    cynic_knowledge::CorpusEdgeKind::EnforcesContract => {
                        enforces.push(edge.target.clone());
                    }
                    cynic_knowledge::CorpusEdgeKind::CitesDocument => {
                        cites.push(edge.target.clone());
                    }
                    cynic_knowledge::CorpusEdgeKind::CrateDependency => {
                        depends_on.push(edge.target.clone());
                    }
                    cynic_knowledge::CorpusEdgeKind::UsesModule => {
                        depends_on.push(edge.target.clone());
                    }
                    _ => {}
                }
            }
            if edge.target == *node {
                match edge.kind {
                    cynic_knowledge::CorpusEdgeKind::EnforcesContract => {
                        enforced_by.push(edge.source.clone());
                    }
                    _ => {}
                }
            }
        }
        let chunk_id = format!("chunk:{}", node);
        ctx_map.insert(
            chunk_id,
            TopologicalContext {
                parent_path: node.clone(),
                parent_authority: String::new(),
                enforces,
                depends_on,
                cites,
                enforced_by,
                depended_on_by,
            },
        );
    }
    ctx_map
}

#[test]
fn topological_retrieval_gain_bm25_vs_topo() {
    let root = repo_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(fixture.schema, "cynic.retrieval_gain_labels.v1");

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    assert!(index.chunk_count() > 1000);

    let manifest = cynic_knowledge::scan_corpus_manifest(&root);
    let graph = KnowledgeGraph::build_from_manifest(&root, &manifest);

    let ctx_map = build_topological_contexts(&graph, &index);

    let mut bm25_mrr_sum = 0.0;
    let mut bm25_ndcg_sum = 0.0;
    let mut graph_mrr_sum = 0.0;
    let mut graph_ndcg_sum = 0.0;
    let mut topo_mrr_sum = 0.0;
    let mut topo_ndcg_sum = 0.0;
    let n = fixture.queries.len() as f64;

    println!(
        "{:<24} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "query", "BM25_MRR", "BM25_NDCG", "GRAPH_MRR", "GRAPH_NDCG", "TOPO_MRR", "TOPO_NDCG"
    );

    for labeled in &fixture.queries {
        let hits = index.search(&labeled.query, TOP_K);
        let grades: Vec<u32> = hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect();
        let rank = grades.iter().position(|grade| *grade >= 1).map(|i| i + 1);
        let bm25_rr = rank.map(|r| 1.0 / r as f64).unwrap_or(0.0);
        let bm25_ndcg = ndcg_at_k(&grades);

        let mut lex_hits: Vec<cynic_knowledge::LexicalHit> = hits
            .iter()
            .map(|h| cynic_knowledge::LexicalHit {
                chunk: h.chunk,
                score: h.score,
            })
            .collect();
        let chunks_by_path = cache.chunks_by_path();

        // Graph expansion
        let graph_expanded =
            cynic_knowledge::expand_hits_with_graph(&lex_hits, &graph, &chunks_by_path, TOP_K);
        let graph_grades: Vec<u32> = graph_expanded
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect();
        let graph_rank = graph_grades.iter().position(|grade| *grade >= 1).map(|i| i + 1);
        let graph_rr = graph_rank.map(|r| 1.0 / r as f64).unwrap_or(0.0);
        let graph_ndcg = ndcg_at_k(&graph_grades);

        graph_mrr_sum += graph_rr;
        graph_ndcg_sum += graph_ndcg;

        // Topology expansion
        let expanded =
            expand_hits_with_topological_contexts(&lex_hits, &ctx_map, &chunks_by_path, "", TOP_K);
        let topo_grades: Vec<u32> = expanded
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect();
        let topo_rank = topo_grades.iter().position(|grade| *grade >= 1).map(|i| i + 1);
        let topo_rr = topo_rank.map(|r| 1.0 / r as f64).unwrap_or(0.0);
        let topo_ndcg = ndcg_at_k(&topo_grades);

        topo_mrr_sum += topo_rr;
        topo_ndcg_sum += topo_ndcg;

        bm25_mrr_sum += bm25_rr;
        bm25_ndcg_sum += bm25_ndcg;

        println!(
            "{:<24} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3}",
            labeled.id, bm25_rr, bm25_ndcg, graph_rr, graph_ndcg, topo_rr, topo_ndcg
        );
    }

    println!();
    println!(
        "AVERAGE: BM25 MRR={:.3} nDCG={:.3} | GRAPH MRR={:.3} nDCG={:.3} | TOPO MRR={:.3} nDCG={:.3}",
        bm25_mrr_sum / n,
        bm25_ndcg_sum / n,
        graph_mrr_sum / n,
        graph_ndcg_sum / n,
        topo_mrr_sum / n,
        topo_ndcg_sum / n
    );
}
