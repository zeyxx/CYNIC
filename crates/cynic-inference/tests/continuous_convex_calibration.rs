//! Continuous Convex Fusion Calibration & Comparison vs Ordinal RRF (Chapter 5, L4)
//!
//! Evaluates candidate continuous fusion strategies against ordinal RRF across the
//! benchmark fixture.
//!
//! Run with:
//! ```bash
//! CYNIC_EMBED_URL="http://127.0.0.1:8081/v1,http://192.168.0.25:8080/v1,http://10.10.10.106:8080/v1" \
//! CYNIC_EMBED_MODEL="multi-endpoint" \
//! CYNIC_EMBED_SUBSET=1000 \
//! cargo test -p cynic-inference --test continuous_convex_calibration -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, FIXTURE_SCHEMA_V1, Fixture, TemplatedEmbedder, TextPipeline, VectorCacheRecord,
    grade_chunk, load_vector_cache, reciprocal_rank, save_vector_cache,
};
use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;
use cynic_knowledge::domain::continuous_fusion::{NormalizationStrategy, fuse_continuous_convex};
use cynic_knowledge::domain::hybrid_rrf::{DEFAULT_RRF_K, fuse_reciprocal_ranks};
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
#[ignore = "needs live embedding endpoints (CYNIC_EMBED_URL)"]
fn test_continuous_convex_calibration_vs_rrf() {
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

    let backend = EmbeddingBackend::from_env().expect("CYNIC_EMBED_URL must be set");
    let bridge = SyncEmbeddingBridge::probe(backend, "CYNIC continuous fusion calibration")
        .expect("embedding probe must succeed");

    let cache_file = root.join("target/eval_corpus_cache.json");
    let subset = std::env::var("CYNIC_EMBED_SUBSET")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);

    let (cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    if subset > 0 && subset < chunks.len() {
        chunks.truncate(subset);
    }

    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "multi-endpoint".into());
    let pipeline = TextPipeline {
        max_chars: 1000,
        doc_prefix: String::new(),
        query_prefix: String::new(),
        doc_template: "{body}".into(),
    };
    let config = BenchConfig {
        subset,
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
        .unwrap_or_else(|| VectorCacheRecord::new(&model, &pipeline.tag()));

    let templated = TemplatedEmbedder::new(&bridge, &pipeline, config.embed_lot, &|_, _| {});
    let (vectors, _reused, _embedded) =
        vectorize_chunks_cached(&templated, &chunks, &mut vector_cache, &|_, _| {})
            .expect("vectorization must succeed");
    save_vector_cache(&vector_cache_path, &vector_cache).expect("save vector cache");

    println!(
        "\n========================================================================================="
    );
    println!(
        "CHAPTER 5: CONTINUOUS CONVEX COMBINATION VS ORDINAL RRF (Corpus Chunks: {})",
        chunks.len()
    );
    println!(
        "========================================================================================="
    );

    let top_k = 5;
    let dense_k = 20;

    // Pre-calculate per-query candidate hits
    struct QueryHits<'a> {
        labeled: &'a cynic_eval::bench::retrieval_gain::LabeledQuery,
        sparse: Vec<cynic_knowledge::SearchHit<'a>>,
        dense: Vec<cynic_knowledge::DenseHit<'a>>,
    }

    let mut evaluated_queries = Vec::new();
    for labeled in &fixture.queries {
        if labeled.trap {
            continue;
        }
        let query_text = config.pipeline.query_text(&labeled.query);
        let query_vectors = bridge
            .embed_batch(&[query_text.as_str()])
            .expect("embed query");
        let query_vector = &query_vectors[0];

        let sparse = index.search(&labeled.query, dense_k);
        let dense = top_k_dense(query_vector, &chunks, &vectors, dense_k);
        evaluated_queries.push(QueryHits {
            labeled,
            sparse,
            dense,
        });
    }

    // 1. Baseline: Pure BM25
    let mrr_bm25: f64 = evaluated_queries
        .iter()
        .map(|q| {
            let grades: Vec<u32> = q
                .sparse
                .iter()
                .take(top_k)
                .map(|h| grade_chunk(h.chunk, &q.labeled.grades))
                .collect();
            reciprocal_rank(&grades)
        })
        .sum::<f64>()
        / evaluated_queries.len() as f64;

    // 2. Baseline: Pure Dense
    let mrr_dense: f64 = evaluated_queries
        .iter()
        .map(|q| {
            let grades: Vec<u32> = q
                .dense
                .iter()
                .take(top_k)
                .map(|h| grade_chunk(h.chunk, &q.labeled.grades))
                .collect();
            reciprocal_rank(&grades)
        })
        .sum::<f64>()
        / evaluated_queries.len() as f64;

    // 3. Baseline: Ordinal RRF (k=60)
    let mrr_rrf: f64 = evaluated_queries
        .iter()
        .map(|q| {
            let fused = fuse_reciprocal_ranks(&q.sparse, &q.dense, DEFAULT_RRF_K);
            let grades: Vec<u32> = fused
                .iter()
                .take(top_k)
                .map(|h| grade_chunk(h.chunk, &q.labeled.grades))
                .collect();
            reciprocal_rank(&grades)
        })
        .sum::<f64>()
        / evaluated_queries.len() as f64;

    println!("BASELINES (N = {} queries):", evaluated_queries.len());
    println!("  - Pure BM25 (top-5):    MRR = {:.4}", mrr_bm25);
    println!("  - Pure Dense (top-5):   MRR = {:.4}", mrr_dense);
    println!("  - Ordinal RRF (k = 60): MRR = {:.4}", mrr_rrf);
    println!(
        "-----------------------------------------------------------------------------------------"
    );

    println!("SWEEP: CONTINUOUS CONVEX COMBINATION S(d) = alpha * Dense + (1 - alpha) * BM25");
    println!("alpha | MinMax MRR | ECDF MRR | Delta vs RRF (MinMax) | Delta vs RRF (ECDF)");

    let alphas = [
        0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80,
    ];
    let mut best_minmax = (0.0, 0.0);
    let mut best_ecdf = (0.0, 0.0);

    for &alpha in &alphas {
        let mrr_minmax: f64 = evaluated_queries
            .iter()
            .map(|q| {
                let fused = fuse_continuous_convex(
                    &q.sparse,
                    &q.dense,
                    alpha,
                    NormalizationStrategy::CandidateMinMax,
                );
                let grades: Vec<u32> = fused
                    .iter()
                    .take(top_k)
                    .map(|h| grade_chunk(h.chunk, &q.labeled.grades))
                    .collect();
                reciprocal_rank(&grades)
            })
            .sum::<f64>()
            / evaluated_queries.len() as f64;

        let mrr_ecdf_score: f64 = evaluated_queries
            .iter()
            .map(|q| {
                let fused = fuse_continuous_convex(
                    &q.sparse,
                    &q.dense,
                    alpha,
                    NormalizationStrategy::EmpiricalCdf,
                );
                let grades: Vec<u32> = fused
                    .iter()
                    .take(top_k)
                    .map(|h| grade_chunk(h.chunk, &q.labeled.grades))
                    .collect();
                reciprocal_rank(&grades)
            })
            .sum::<f64>()
            / evaluated_queries.len() as f64;

        if mrr_minmax > best_minmax.1 {
            best_minmax = (alpha, mrr_minmax);
        }
        if mrr_ecdf_score > best_ecdf.1 {
            best_ecdf = (alpha, mrr_ecdf_score);
        }

        println!(
            "{:5.2} |   {:.4}   |  {:.4}  |       {:+0.4}         |      {:+0.4}",
            alpha,
            mrr_minmax,
            mrr_ecdf_score,
            mrr_minmax - mrr_rrf,
            mrr_ecdf_score - mrr_rrf
        );
    }

    println!(
        "-----------------------------------------------------------------------------------------"
    );
    println!(
        "OPTIMAL: MinMax alpha* = {:.2} (MRR = {:.4}), ECDF alpha* = {:.2} (MRR = {:.4})",
        best_minmax.0, best_minmax.1, best_ecdf.0, best_ecdf.1
    );

    // 4. Test Bilateral Guard with Continuous Convex Combination
    println!(
        "-----------------------------------------------------------------------------------------"
    );
    println!("BILATERAL GUARD + CONTINUOUS CONVEX (bm25_gap = 0.10, dense_gap = 0.05):");
    for &alpha in &[0.05, 0.10, 0.15, 0.20, 0.30] {
        let mrr_guarded_minmax: f64 = evaluated_queries
            .iter()
            .map(|q| {
                let bm25_conf = cynic_knowledge::bm25_margin(&q.sparse) >= 0.10;
                let dense_conf = cynic_knowledge::dense_margin(&q.dense) >= 0.05;

                let final_hits = if bm25_conf && !dense_conf {
                    q.sparse.clone()
                } else {
                    fuse_continuous_convex(
                        &q.sparse,
                        &q.dense,
                        alpha,
                        NormalizationStrategy::CandidateMinMax,
                    )
                };

                let grades: Vec<u32> = final_hits
                    .iter()
                    .take(top_k)
                    .map(|h| grade_chunk(h.chunk, &q.labeled.grades))
                    .collect();
                reciprocal_rank(&grades)
            })
            .sum::<f64>()
            / evaluated_queries.len() as f64;

        println!(
            "  alpha = {:4.2} + Bilateral Guard -> MRR = {:.4} (vs Pure BM25: {:+0.4}, vs RRF: {:+0.4})",
            alpha,
            mrr_guarded_minmax,
            mrr_guarded_minmax - mrr_bm25,
            mrr_guarded_minmax - mrr_rrf
        );
    }
    println!(
        "=========================================================================================\n"
    );
}
