//! Neural hybrid-gain measurement — ignored by default, needs a live endpoint.
//!
//! Thin entry over `cynic_eval::bench::retrieval_gain`: this file owns the
//! environment (config values), the embedding backend (bridge), and the
//! report file layout. All orchestration — corpus sync, cached vectorize,
//! triple measurement, grading — lives in the bench module, where it is
//! unit-tested and reusable by `cynic know eval`.
//!
//! Run locally (same machine as the embedding server, no tunnel):
//! `CYNIC_EMBED_URL=http://127.0.0.1:8081/v1 \
//!  CYNIC_EMBED_MODEL=cortex-embed \
//!  cargo test -p cynic-inference --test neural_hybrid_gain_live -- --ignored --nocapture`
//!
//! Levers (all optional, defaults = status-quo full run):
//! `CYNIC_EMBED_SUBSET` (0 = all), `CYNIC_EMBED_MAX_CHARS` (default 1000),
//! `CYNIC_EMBED_TRUNC_DIM` (0 = full), `CYNIC_EMBED_DOC_PREFIX`,
//! `CYNIC_EMBED_QUERY_PREFIX`, `CYNIC_EMBED_DOC_TEMPLATE` (default `{body}`),
//! `CYNIC_EMBED_TOP_K` (default 5), `CYNIC_EMBED_DENSE_K` (default 20),
//! `CYNIC_EMBED_LOT` (default 32), `CYNIC_FUSION_GUARD` (absent = plain RRF;
//! 0.0..=1.0 = BM25 top1-top2 margin at or above which sparse wins alone).

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, FIXTURE_SCHEMA_V1, Fixture, TextPipeline, VectorCacheRecord, load_vector_cache,
    run_bench, save_vector_cache,
};
use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing contracts/");
        }
    }
    dir
}

fn env_usize(name: &str, min: usize, max: usize, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .filter(|value| (min..=max).contains(value))
        .unwrap_or(default)
}

fn env_guard_gap() -> Option<f64> {
    std::env::var("CYNIC_FUSION_GUARD")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .filter(|gap: &f64| (0.0..=1.0).contains(gap))
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
        fixture.schema, FIXTURE_SCHEMA_V1,
        "fixture schema marker must match"
    );

    let pipeline = TextPipeline {
        max_chars: env_usize("CYNIC_EMBED_MAX_CHARS", 256, 8000, 1000),
        doc_prefix: std::env::var("CYNIC_EMBED_DOC_PREFIX").unwrap_or_default(),
        query_prefix: std::env::var("CYNIC_EMBED_QUERY_PREFIX").unwrap_or_default(),
        doc_template: std::env::var("CYNIC_EMBED_DOC_TEMPLATE").unwrap_or_else(|_| "{body}".into()),
    };
    let config = BenchConfig {
        subset: env_usize("CYNIC_EMBED_SUBSET", 1, usize::MAX, 0),
        trunc_dim: env_usize("CYNIC_EMBED_TRUNC_DIM", 32, 4096, 0),
        top_k: env_usize("CYNIC_EMBED_TOP_K", 1, 50, 5),
        dense_k: env_usize("CYNIC_EMBED_DENSE_K", 1, 200, 20),
        guard_gap: env_guard_gap(),
        embed_lot: env_usize("CYNIC_EMBED_LOT", 1, 128, 32),
        pipeline,
        model: std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "unknown-model".into()),
    };

    let backend = EmbeddingBackend::from_env().expect("CYNIC_EMBED_URL must be set");
    let bridge = SyncEmbeddingBridge::probe(backend, "CYNIC neural hybrid probe")
        .expect("embedding probe must succeed");
    println!("NEURAL_BACKEND dims={}", bridge.dimensions());

    let mut vector_cache = load_vector_cache(
        &root.join(config.vector_cache_filename()),
        &config.model,
        &config.pipeline.tag(),
    )
    .unwrap_or_else(|| VectorCacheRecord::new(&config.model, &config.pipeline.tag()));

    let corpus_start = std::time::Instant::now();
    let last_report = std::cell::Cell::new(0usize);
    let report = run_bench(
        &root,
        &root.join("target/eval_corpus_cache.json"),
        &fixture,
        &config,
        &bridge,
        &mut vector_cache,
        &|done, total| {
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
    .expect("bench must measure");
    save_vector_cache(&root.join(config.vector_cache_filename()), &vector_cache)
        .expect("persist vector cache");

    println!(
        "NEURAL_CORPUS chunks={} (subset={})",
        report.corpus_chunks,
        if config.subset > 0 {
            config.subset.to_string()
        } else {
            "all".into()
        }
    );
    println!(
        "EMBED done: {} vectors ({} reused, {} embedded) in {:.0}s",
        report.corpus_chunks, report.vectors_reused, report.vectors_embedded, report.embed_secs
    );
    print!("{}", report.table());
    let _ = std::fs::write(
        root.join("target/neural_hybrid_gain.json"),
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
}
