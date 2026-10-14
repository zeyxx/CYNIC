//! Neural hybrid-gain measurement — ignored by default, needs the live LXC endpoint.
//!
//! Run on the builder (same LAN as cynic-lxc-embedding):
//! `CYNIC_EMBED_URL=http://10.10.10.106:8080/v1 \
//!  CYNIC_EMBED_MODEL=embeddinggemma-2-bf16 \
//!  cargo test -p cynic-inference --test neural_hybrid_gain_live -- --ignored --nocapture`
//!
//! Compares BM25-only MRR against BM25 + neural-dense RRF on the graded
//! fixture. This is the run that decides whether the neural backend earns
//! its keep: the hashed-token stub measured delta -0.089, so a non-positive
//! delta here means the wiring is proven but the model adds no retrieval
//! value on this corpus.

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;
use cynic_knowledge::{ChunkEmbedder, ChunkVector, sync_workspace_corpus};
use serde::Deserialize;

const TOP_K: usize = 5;
const DENSE_K: usize = 20;
/// Server batch ceiling (llama-server physical batch 512 tokens): inputs are
/// front-truncated to stay under it. 1000 chars ≈ 370 tokens even on dense
/// code chunks (~2.7 chars/token observed live). A measurement decision, not
/// bridge behavior — the bridge must surface oversize inputs as errors.
const MAX_EMBED_CHARS: usize = 1000;

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

fn truncate_front(text: &str) -> &str {
    if text.len() <= MAX_EMBED_CHARS {
        text
    } else {
        let mut end = MAX_EMBED_CHARS;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    }
}

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing contracts/");
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
#[ignore = "needs a live /v1/embeddings endpoint (CYNIC_EMBED_URL)"]
fn neural_hybrid_gain_over_bm25_is_measured() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(
        fixture.schema, "cynic.retrieval_gain_labels.v1",
        "fixture schema marker must match"
    );

    let backend = EmbeddingBackend::from_env().expect("CYNIC_EMBED_URL must be set");
    let bridge = SyncEmbeddingBridge::probe(backend, "CYNIC neural hybrid probe")
        .expect("embedding probe must succeed");
    println!("NEURAL_BACKEND dims={}", bridge.dimensions());

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    assert!(
        index.chunk_count() > 1000,
        "corpus must index over 1000 chunks"
    );

    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    let corpus_start = std::time::Instant::now();
    let last_report = std::cell::Cell::new(0usize);
    let raw_vectors = bridge
        .embed_batch_reported(
            &chunks
                .iter()
                .map(|c| truncate_front(c.content.as_str()))
                .collect::<Vec<_>>(),
            &|done, total| {
                // Throttle to every ~10 lots: one line per ~320 chunks.
                if done >= last_report.get() + 320 || done == total {
                    last_report.set(done);
                    println!(
                        "EMBED {done}/{total} chunks ({}%) elapsed={:.0}s",
                        done * 100 / total,
                        corpus_start.elapsed().as_secs_f32()
                    );
                }
            },
        )
        .expect("corpus vectors");
    println!(
        "EMBED done: {} vectors in {:.0}s",
        raw_vectors.len(),
        corpus_start.elapsed().as_secs_f32()
    );
    let vectors: Vec<ChunkVector> = chunks
        .iter()
        .zip(raw_vectors)
        .map(|(chunk, vector)| ChunkVector {
            chunk_id: chunk.chunk_id.clone(),
            vector,
        })
        .collect();

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
        let query_vector = bridge
            .embed_batch(std::slice::from_ref(&truncate_front(&labeled.query)))
            .expect("query vector");
        let hybrid_hits =
            index.search_hybrid(&labeled.query, &query_vector[0], &vectors, TOP_K, DENSE_K);
        assert!(
            !hybrid_hits.is_empty(),
            "[{}] hybrid must retrieve at least one hit",
            labeled.id
        );

        let bm25_rr = reciprocal_rank(
            &bm25_hits
                .iter()
                .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
                .collect::<Vec<_>>(),
        );
        let hybrid_rr = reciprocal_rank(
            &hybrid_hits
                .iter()
                .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
                .collect::<Vec<_>>(),
        );
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
    }

    assert!(measured > 0, "fixture must contain non-trap queries");
    let mean_bm25 = sum_bm25 / measured as f64;
    let mean_hybrid = sum_hybrid / measured as f64;
    println!(
        "NEURAL_MEAN MRR over {measured} queries: bm25={mean_bm25:.3} hybrid={mean_hybrid:.3} delta={:.3}",
        mean_hybrid - mean_bm25
    );
    let report = serde_json::json!({
        "schema": "cynic.neural_hybrid_gain.v1",
        "measured_queries": measured,
        "mean_mrr_bm25": mean_bm25,
        "mean_mrr_hybrid": mean_hybrid,
        "delta_hybrid_minus_bm25": mean_hybrid - mean_bm25,
    });
    let _ = std::fs::write(
        root.join("target/neural_hybrid_gain.json"),
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
}
