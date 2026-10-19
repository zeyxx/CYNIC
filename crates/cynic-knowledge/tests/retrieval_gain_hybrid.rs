//! Hybrid-gain measurement: BM25-only vs BM25 + dense RRF on the graded fixture.
//!
//! The dense backend here is a deterministic hashed-token embedder — it
//! measures the *fusion mechanics* (RRF over two rank lists), not a neural
//! semantic model. A neural backend plugs in later through `ChunkEmbedder`
//! and must beat both numbers printed here. The gain direction is therefore
//! recorded warn-only: asserting hybrid superiority over a stub backend
//! would bless noise.

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_knowledge::{ChunkEmbedder, sync_workspace_corpus, tokenize, vectorize_chunks};
use serde::Deserialize;

const TOP_K: usize = 5;
const DENSE_K: usize = 20;
const DIMS: usize = 64;

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

/// Deterministic hashed-token embedder (measurement backend, not semantic).
/// Token counts folded into `DIMS` buckets by stable FNV-1a, L2-normalized.
struct HashedTokenEmbedder;

impl ChunkEmbedder for HashedTokenEmbedder {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|text| embed_text(text)).collect())
    }
}

fn embed_text(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0f32; DIMS];
    for token in tokenize(text) {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in token.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        vec[(hash % DIMS as u64) as usize] += 1.0;
    }
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut vec {
            *x /= norm;
        }
    }
    vec
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

fn reciprocal_rank(grades_ranked: &[u32]) -> f64 {
    grades_ranked
        .iter()
        .position(|grade| *grade >= 1)
        .map(|i| 1.0 / (i + 1) as f64)
        .unwrap_or(0.0)
}

#[test]
fn hybrid_rrf_gain_over_bm25_is_measured() {
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

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    assert!(
        index.chunk_count() > 1000,
        "corpus must index over 1000 chunks"
    );

    // Deterministic chunk order: join vectors by chunk_id, never by position.
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    let embedder = HashedTokenEmbedder;
    let vectors = vectorize_chunks(&embedder, &chunks).expect("chunk vectors");

    println!(
        "{:<24} {:>8} {:>8}  top1-hybrid",
        "query", "MRR-bm25", "MRR-hybr"
    );
    let mut sum_bm25 = 0.0;
    let mut sum_hybrid = 0.0;
    let mut measured = 0usize;
    for labeled in &fixture.queries {
        if labeled.trap {
            continue;
        }
        let bm25_hits = index.search(&labeled.query, TOP_K);
        assert!(
            !bm25_hits.is_empty(),
            "[{}] valid query must retrieve at least one hit",
            labeled.id
        );
        let query_vector = embed_text(&labeled.query);
        let hybrid_hits =
            index.search_hybrid(&labeled.query, &query_vector, &vectors, TOP_K, DENSE_K);
        assert!(
            !hybrid_hits.is_empty(),
            "[{}] hybrid must retrieve at least one hit",
            labeled.id
        );

        let bm25_grades: Vec<u32> = bm25_hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect();
        let hybrid_grades: Vec<u32> = hybrid_hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect();
        let bm25_rr = reciprocal_rank(&bm25_grades);
        let hybrid_rr = reciprocal_rank(&hybrid_grades);
        sum_bm25 += bm25_rr;
        sum_hybrid += hybrid_rr;
        measured += 1;
        let top1 = hybrid_hits
            .first()
            .map(|hit| hit.chunk.source_path.as_str())
            .unwrap_or("<none>");
        println!(
            "{:<24} {:>8.3} {:>8.3}  {top1}",
            labeled.id, bm25_rr, hybrid_rr
        );

        // Determinism: identical hybrid order on rerun.
        let rerun = index.search_hybrid(&labeled.query, &query_vector, &vectors, TOP_K, DENSE_K);
        let order: Vec<&str> = hybrid_hits
            .iter()
            .map(|hit| hit.chunk.chunk_id.as_str())
            .collect();
        let rerun_order: Vec<&str> = rerun
            .iter()
            .map(|hit| hit.chunk.chunk_id.as_str())
            .collect();
        assert_eq!(
            order, rerun_order,
            "[{}] hybrid retrieval must be deterministic",
            labeled.id
        );
    }

    assert!(measured > 0, "fixture must contain non-trap queries");
    let mean_bm25 = sum_bm25 / measured as f64;
    let mean_hybrid = sum_hybrid / measured as f64;
    println!(
        "MEAN MRR over {measured} queries: bm25={mean_bm25:.3} hybrid={mean_hybrid:.3} delta={:.3}",
        mean_hybrid - mean_bm25
    );
    // Persist the means beside the harness cache so a proof run leaves
    // readable evidence even though passing-test stdout is captured.
    let report = serde_json::json!({
        "schema": "cynic.hybrid_gain_measure.v1",
        "measured_queries": measured,
        "mean_mrr_bm25": mean_bm25,
        "mean_mrr_hybrid": mean_hybrid,
        "delta_hybrid_minus_bm25": mean_hybrid - mean_bm25,
    });
    let _ = std::fs::write(
        root.join("target/hybrid_gain_measure.json"),
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
}
