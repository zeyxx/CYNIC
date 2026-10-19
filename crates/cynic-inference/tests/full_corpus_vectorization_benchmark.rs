//! Full Corpus Vectorization Benchmark (10,645 chunks) over 3 Endpoints (Chapter 7, L6)
//!
//! Measures global throughput across the distributed Vulkan cluster.
//!
//! Run with:
//! ```bash
//! CYNIC_EMBED_URL="http://127.0.0.1:8081/v1#4,http://192.168.0.25:8080/v1#4,http://10.10.10.106:8080/v1#2" \
//! CYNIC_EMBED_MODEL="multi-endpoint" \
//! cargo test -p cynic-inference --test full_corpus_vectorization_benchmark -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, TemplatedEmbedder, TextPipeline, VectorCacheRecord, load_vector_cache,
    save_vector_cache,
};
use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;
use cynic_knowledge::{sync_workspace_corpus, vectorize_chunks_cached};

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
#[ignore = "needs live embedding endpoints (CYNIC_EMBED_URL)"]
fn test_vectorize_full_workspace_corpus() {
    let root = workspace_root();
    let backend = EmbeddingBackend::from_env().expect("CYNIC_EMBED_URL must be set");
    let engine = backend.base_urls().join(",");
    let bridge = SyncEmbeddingBridge::probe(backend, "CYNIC full corpus vectorization")
        .expect("embedding probe must succeed");

    let cache_file = root.join("target/eval_corpus_cache.json");
    // subset 0 = full corpus
    let (cache, _index) = sync_workspace_corpus(&root, Some(&cache_file), 0);
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));

    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "multi-endpoint".into());
    let pipeline = TextPipeline {
        max_chars: 1000,
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
    let mut vector_cache = load_vector_cache(&vector_cache_path, &model, &pipeline.tag(), &engine)
        .unwrap_or_else(|| VectorCacheRecord::new(&model, &pipeline.tag(), &engine));

    println!(
        "\n========================================================================================="
    );
    println!(
        "DISTRIBUTED FULL CORPUS VECTORIZATION (Total Chunks: {})",
        chunks.len()
    );
    println!(
        "========================================================================================="
    );

    let start = Instant::now();
    let templated = TemplatedEmbedder::new(&bridge, &pipeline, config.embed_lot, &|done, total| {
        if done % 500 == 0 || done == total {
            print!(
                "\r  -> Progress: {}/{} chunks ({:.1}%)",
                done,
                total,
                (done as f64 / total as f64) * 100.0
            );
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    });

    let (vectors, reused, embedded) =
        vectorize_chunks_cached(&templated, &chunks, &mut vector_cache, &|_, _| {})
            .expect("vectorization must succeed");
    save_vector_cache(&vector_cache_path, &vector_cache).expect("save vector cache");

    let elapsed = start.elapsed();
    let throughput = if embedded > 0 {
        embedded as f64 / elapsed.as_secs_f64()
    } else {
        chunks.len() as f64 / elapsed.as_secs_f64()
    };

    println!(
        "\n-----------------------------------------------------------------------------------------"
    );
    println!(
        "VECTORIZATION COMPLETED:\n  - Total Vectors: {}\n  - Reused (Cache): {}\n  - Freshly Embedded: {}\n  - Wall Time: {:.2}s\n  - Throughput: {:.1} chunks/s",
        vectors.len(),
        reused,
        embedded,
        elapsed.as_secs_f64(),
        throughput
    );
    println!(
        "=========================================================================================\n"
    );

    assert_eq!(vectors.len(), chunks.len(), "all chunks must be vectorized");
}
