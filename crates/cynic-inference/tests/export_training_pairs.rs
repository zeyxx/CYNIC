//! Contrastive training-pair export (SOTA rung 0: data asset, no training yet).
//!
//! Mines hard negatives from our own misses: for each non-trap fixture
//! query, positives are chunks the bench itself grades >= 1 (`grade_chunk`
//! — chunk id, span, then path), hard negatives are the top dense hits the
//! bench grades 0. Output is a versioned JSON fixture that a future
//! contrastive fine-tune (or cross-encoder rerank eval) consumes directly.
//!
//! Run with:
//! ```bash
//! cargo test -p cynic-inference --test export_training_pairs -- --ignored --nocapture
//! ```
//! Endpoints resolve from the environment or `infra/registry.json`
//! (`EmbeddingBackend::from_env`). Corpus vectors reuse the Q8-uniform
//! cache, so only the 85 query texts are embedded live.

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, FIXTURE_SCHEMA_V1, Fixture, TemplatedEmbedder, TextPipeline, VectorCacheRecord,
    BenchConfig, FIXTURE_SCHEMA_V1, Fixture, TemplatedEmbedder, TextPipeline,
    grade_chunk, load_vector_cache,
};
use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;
use cynic_knowledge::{ChunkEmbedder, sync_workspace_corpus, top_k_dense, vectorize_chunks_cached};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing contracts/");
        }
    }
    dir
}

#[test]
#[ignore = "needs live embedding endpoints; writes the training fixture"]
fn test_export_training_pairs() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(
        fixture.schema, FIXTURE_SCHEMA_V1,
        "fixture schema marker must match"
    );

    let backend = EmbeddingBackend::from_env().expect("embedding endpoints must resolve");
    let engine = backend.base_urls().join(",");
    let bridge = SyncEmbeddingBridge::probe(backend, "CYNIC training-pair export")
        .expect("embedding probe must succeed");

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (cache, _index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));

    // Identical pipeline to the frozen Q8 run: cache fully reused, zero
    // corpus embeds, only query texts go over the wire.
    let model = "q8-uniform".to_string();
    let pipeline = TextPipeline {
        max_chars: 2500,
        doc_prefix: String::new(),
        query_prefix: String::new(),
        doc_template: "{body}".into(),
    };
    let config = BenchConfig {
        subset: 0,
        trunc_dim: 0,
        top_k: 5,
        dense_k: 20,
        guard_gap: None,
        dense_guard_gap: None,
        embed_lot: 32,
        pipeline: pipeline.clone(),
        model: model.clone(),
    };
    let vector_cache_path = root.join(config.vector_cache_filename());
    let mut vector_cache = load_vector_cache(&vector_cache_path, &model, &pipeline.tag())
    let mut vector_cache = load_vector_cache(&vector_cache_path, &model, &pipeline.tag(), &engine)
        .expect("Q8-uniform vector cache must exist from the frozen run");
    let templated = TemplatedEmbedder::new(&bridge, &pipeline, config.embed_lot, &|_, _| {});
    let (vectors, _reused, embedded) =
        vectorize_chunks_cached(&templated, &chunks, &mut vector_cache, &|_, _| {})
            .expect("vectorization must succeed");
    if embedded > 0 {
        println!(
            "corpus drift since frozen run: {embedded} new chunks embedded live (docs edited after the Q8 run); frozen cache file untouched"
        );
    }

    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let mut lines: Vec<String> = Vec::new();
    lines.push(
        serde_json::json!({
            "kind": "provenance",
            "head": head,
            "corpus_chunks": chunks.len(),
            "model": model,
            "pipeline": pipeline.tag(),
            "positives": "grade_chunk >= 1 (chunk id, span, then path — the bench's own grading)",
            "hard_negatives": "top dense hits graded 0, deepest pool first",
            "drift_embedded_live": embedded,
        })
        .to_string(),
    );

    let mut query_count = 0;
    let mut pos_total = 0;
    let mut neg_total = 0;
    let mut thin_queries: Vec<String> = Vec::new();
    for labeled in &fixture.queries {
        if labeled.trap {
            continue;
        }
        let query_text = config.pipeline.query_text(&labeled.query);
        let query_vector = &bridge
            .embed_batch(&[query_text.as_str()])
            .expect("embed query")[0];
        let dense = top_k_dense(query_vector, &chunks, &vectors, 30);

        let positives: Vec<serde_json::Value> = chunks
            .iter()
            .filter(|c| grade_chunk(c, &labeled.grades) >= 1)
            .map(|c| {
                serde_json::json!({
                    "chunk_id": c.chunk_id,
                    "path": c.source_path,
                    "lines": [c.line_start, c.line_end],
                    "text": c.content,
                })
            })
            .collect();
        let negatives: Vec<serde_json::Value> = dense
            .iter()
            .filter(|h| grade_chunk(h.chunk, &labeled.grades) == 0)
            .take(10)
            .map(|h| {
                serde_json::json!({
                    "chunk_id": h.chunk.chunk_id,
                    "path": h.chunk.source_path,
                    "lines": [h.chunk.line_start, h.chunk.line_end],
                    "dense_rank": dense.iter().position(|x| x.chunk.chunk_id == h.chunk.chunk_id).unwrap_or(usize::MAX) + 1,
                    "text": h.chunk.content,
                })
            })
            .collect();

        query_count += 1;
        pos_total += positives.len();
        neg_total += negatives.len();
        if negatives.len() < 5 {
            thin_queries.push(labeled.id.clone());
        }
        lines.push(
            serde_json::json!({
                "kind": "pair",
                "query_id": labeled.id,
                "query": labeled.query,
                "positives": positives,
                "hard_negatives": negatives,
            })
            .to_string(),
        );
    }

    let out = root.join("crates/cynic-knowledge/tests/retrieval_training_pairs.json");
    let array: Vec<serde_json::Value> = lines
        .iter()
        .map(|l| serde_json::from_str(l).expect("pair line is valid JSON"))
        .collect();
    std::fs::write(
        &out,
        serde_json::to_string_pretty(&array).expect("encode pairs"),
    )
    .expect("write training pairs");
    println!("queries: {query_count}, positives: {pos_total}, hard negatives: {neg_total}");
    println!("queries with <5 negatives (saturated, skip in training): {thin_queries:?}");
    println!("wrote {}", out.display());
    assert!(query_count >= 80, "fixture must keep its measured size");
    assert!(
        neg_total > query_count,
        "most queries must contribute negatives"
    );
}
