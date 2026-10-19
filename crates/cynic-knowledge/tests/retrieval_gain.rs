//! Retrieval-gain bench fixture runner (11.10 tranche 1: BM25 baseline).
//!
//! Reads the graded labels in `retrieval_gain_labels.json`, runs BM25-only
//! retrieval, and reports MRR + nDCG@5 per query. This records the baseline
//! the hybrid path (slice 3) must beat on this exact fixture — the comparison
//! assert lands with the hybrid wiring, not here.

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_knowledge::sync_workspace_corpus;
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
    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

#[test]
fn retrieval_gain_baseline_bm25_is_recorded() {
    let root = repo_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(
        fixture.schema, "cynic.retrieval_gain_labels.v1",
        "fixture schema marker must match"
    );
    assert!(
        !fixture.queries.is_empty(),
        "fixture without queries measures nothing"
    );
    for labeled in &fixture.queries {
        assert!(
            labeled.grades.values().all(|grade| *grade <= 3),
            "[{}] grades are bounded 0..=3 (gain formula overflows above)",
            labeled.id
        );
    }

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (_cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    assert!(
        index.chunk_count() > 1000,
        "corpus must index over 1000 chunks"
    );

    println!("{:<24} {:>6} {:>8}  top1", "query", "MRR", "nDCG@5");
    for labeled in &fixture.queries {
        let hits = index.search(&labeled.query, TOP_K);
        let grades: Vec<u32> = hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect();
        let rank = grades.iter().position(|grade| *grade >= 1).map(|i| i + 1);
        let rr = rank.map(|r| 1.0 / r as f64).unwrap_or(0.0);
        let ndcg = ndcg_at_k(&grades);
        let top1 = hits
            .first()
            .map(|hit| hit.chunk.source_path.as_str())
            .unwrap_or("<none>");
        println!("{:<24} {:>6.3} {:>8.3}  {top1}", labeled.id, rr, ndcg);

        if !labeled.trap {
            assert!(
                !hits.is_empty(),
                "[{}] valid query must retrieve at least one hit",
                labeled.id
            );
            // Determinism: same index, same query, same order.
            let rerun = index.search(&labeled.query, TOP_K);
            let order: Vec<&str> = hits.iter().map(|hit| hit.chunk.chunk_id.as_str()).collect();
            let rerun_order: Vec<&str> = rerun
                .iter()
                .map(|hit| hit.chunk.chunk_id.as_str())
                .collect();
            assert_eq!(
                order, rerun_order,
                "[{}] retrieval must be deterministic",
                labeled.id
            );
        }
    }
}
