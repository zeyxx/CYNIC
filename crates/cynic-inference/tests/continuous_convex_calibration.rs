//! Continuous Convex Fusion Calibration & Comparison vs Ordinal RRF (Chapter 5, L4)
//!
//! Evaluates candidate continuous fusion strategies against ordinal RRF across the
//! benchmark fixture.
//!
//! Run with:
//! ```bash
//! CYNIC_EMBED_URL="http://127.0.0.1:8081/v1#slots=2,http://192.168.0.25:8080/v1#slots=2" \
//! CYNIC_EMBED_MODEL="multi-endpoint" \
//! CYNIC_EMBED_SUBSET=0 \
//! cargo test -p cynic-inference --test continuous_convex_calibration -- --ignored --nocapture
//! ```
//! `CYNIC_EMBED_SUBSET=0` scores the whole corpus; any other value takes the
//! first-N chunks by content-hash order (uniform sample, not a prefix).
//!
//! Physical slot law: each runner enforces `n_ctx / --parallel` tokens per
//! input (cortex `-c 4096 --parallel 2` and compute-01 `-c 4096 --parallel 2`
//! give 2 048-token slots; measured corpus max is 1 101 tokens). lxc106 is
//! excluded: no admin access, ceiling unmeasurable, fails closed on long inputs.

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, FIXTURE_SCHEMA_V1, Fixture, TemplatedEmbedder, TextPipeline, VectorCacheRecord,
    grade_chunk, load_vector_cache, reciprocal_rank, save_vector_cache, wilcoxon_signed_rank,
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
    let engine = backend.base_urls().join(",");
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
    // Dimension screen: truncate cached vectors to the leading N dims
    // (Matryoshka). 0 = full dim. Cosine normalizes, so no renormalization
    // step is needed for ranking.
    let trunc_dim = std::env::var("CYNIC_TRUNC_DIM")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    // Zero-truncation window: measured corpus max is 2 498 chars, so every
    // chunk reaches the dense model intact (fits the 2 048-token runners).
    // Pipeline from env (model-card protocols as data): prefixes off by
    // default; any value lands in the cache tag, so prefixed vectors never
    // mix with bare ones.
    let pipeline = TextPipeline {
        max_chars: 2500,
        doc_prefix: std::env::var("CYNIC_DOC_PREFIX").unwrap_or_default(),
        query_prefix: std::env::var("CYNIC_QUERY_PREFIX").unwrap_or_default(),
        doc_template: std::env::var("CYNIC_DOC_TEMPLATE").unwrap_or("{body}".into()),
    };
    let config = BenchConfig {
        subset,
        trunc_dim,
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
    let mut vector_cache = load_vector_cache(&vector_cache_path, &model, &pipeline.tag(), &engine)
        .unwrap_or_else(|| VectorCacheRecord::new(&model, &pipeline.tag(), &engine));

    let templated = TemplatedEmbedder::new(&bridge, &pipeline, config.embed_lot, &|done, total| {
        if done % 320 == 0 || done == total {
            eprintln!("embed {done}/{total}");
        }
    });
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

    // 1-3. Baselines with per-query reciprocal ranks retained for paired tests.
    let rr_bm25: Vec<f64> = evaluated_queries
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
        .collect();
    let rr_dense: Vec<f64> = evaluated_queries
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
        .collect();
    let rr_rrf: Vec<f64> = evaluated_queries
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
        .collect();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let (mrr_bm25, mrr_dense, mrr_rrf) = (mean(&rr_bm25), mean(&rr_dense), mean(&rr_rrf));

    println!("BASELINES (N = {} queries):", evaluated_queries.len());
    println!("  - Pure BM25 (top-5):    MRR = {:.4}", mrr_bm25);
    println!("  - Pure Dense (top-5):   MRR = {:.4}", mrr_dense);
    println!("  - Ordinal RRF (k = 60): MRR = {:.4}", mrr_rrf);
    println!(
        "-----------------------------------------------------------------------------------------"
    );

    println!("SWEEP: CONTINUOUS CONVEX COMBINATION S(d) = alpha * Dense + (1 - alpha) * BM25");
    println!("alpha | MinMax MRR | ECDF MRR | Delta vs RRF (MinMax) | Delta vs RRF (ECDF)");

    // Full non-linearity sweep: alpha in [0.00, 1.00] at 0.05 resolution.
    let alphas: Vec<f64> = (0..=20).map(|i| i as f64 * 0.05).collect();
    let mut best_minmax = (0.0, 0.0);
    let mut best_ecdf = (0.0, 0.0);
    let mut sweep_rows: Vec<(f64, f64, f64)> = Vec::with_capacity(alphas.len());
    // Per-query ECDF reciprocal ranks per alpha, for the paired test.
    let mut ecdf_rr_by_alpha: Vec<(f64, Vec<f64>)> = Vec::with_capacity(alphas.len());
    // Per-query MinMax reciprocal ranks per alpha, for per-doctype routing.
    let mut minmax_rr_by_alpha: Vec<(f64, Vec<f64>)> = Vec::with_capacity(alphas.len());

    for &alpha in &alphas {
        let rr_minmax: Vec<f64> = evaluated_queries
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
            .collect();

        let rr_ecdf: Vec<f64> = evaluated_queries
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
            .collect();

        let (mrr_minmax, mrr_ecdf_score) = (mean(&rr_minmax), mean(&rr_ecdf));
        if mrr_minmax > best_minmax.1 {
            best_minmax = (alpha, mrr_minmax);
        }
        if mrr_ecdf_score > best_ecdf.1 {
            best_ecdf = (alpha, mrr_ecdf_score);
        }
        sweep_rows.push((alpha, mrr_minmax, mrr_ecdf_score));
        ecdf_rr_by_alpha.push((alpha, rr_ecdf));
        minmax_rr_by_alpha.push((alpha, rr_minmax));

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

    // 4. Paired Wilcoxon signed-rank: best ECDF fusion vs ordinal RRF.
    let best_rr_ecdf: &[f64] = &ecdf_rr_by_alpha
        .iter()
        .find(|(a, _)| *a == best_ecdf.0)
        .expect("best ECDF alpha must be in sweep")
        .1;
    let paired_diffs: Vec<f64> = best_rr_ecdf
        .iter()
        .zip(rr_rrf.iter())
        .map(|(fused, rrf)| fused - rrf)
        .collect();
    let wilcoxon = wilcoxon_signed_rank(&paired_diffs);
    let wins = paired_diffs.iter().filter(|d| **d > 0.0).count();
    let losses = paired_diffs.iter().filter(|d| **d < 0.0).count();
    let ties = paired_diffs.len() - wins - losses;
    println!(
        "WILCOXON (paired, best ECDF alpha={:.2} vs RRF, N={} queries): W+ = {:.1}, n_nonzero = {}, wins/losses/ties = {}/{}/{}, z = {:.3}, p_two_sided = {:.4} ({})",
        best_ecdf.0,
        paired_diffs.len(),
        wilcoxon.w_plus,
        wilcoxon.n_nonzero,
        wins,
        losses,
        ties,
        wilcoxon.z,
        wilcoxon.p_two_sided,
        if wilcoxon.p_two_sided < 0.05 {
            "SIGNIFICANT at 0.05"
        } else {
            "not significant at 0.05"
        }
    );

    // 5. Machine-readable sweep report for the SSOT contract.
    let report = serde_json::json!({
        // v2 adds the full alpha-per-query matrix plus per-query gold_doctype
        // for per-data-type routing; v1 archives keep only best columns.
        "schema": "cynic.convex_sweep_report.v2",
        "corpus_chunks_scored": chunks.len(),
        "measured_queries": evaluated_queries.len(),
        "pipeline_tag": pipeline.tag(),
        "model": model,
        "endpoints_env": std::env::var("CYNIC_EMBED_URL").unwrap_or_default(),
        "subset_env": subset,
        "top_k": top_k,
        "dense_k": dense_k,
        "rrf_k": DEFAULT_RRF_K,
        "mrr_bm25": mrr_bm25,
        "mrr_dense": mrr_dense,
        "mrr_rrf": mrr_rrf,
        "sweep": sweep_rows.iter().map(|(a, mm, ec)| {
            serde_json::json!({"alpha": a, "mrr_minmax": mm, "mrr_ecdf": ec})
        }).collect::<Vec<_>>(),
        "best_minmax": {"alpha": best_minmax.0, "mrr": best_minmax.1},
        "best_ecdf": {"alpha": best_ecdf.0, "mrr": best_ecdf.1},
        "wilcoxon_best_ecdf_vs_rrf": {
            "n_nonzero": wilcoxon.n_nonzero,
            "w_plus": wilcoxon.w_plus,
            "z": wilcoxon.z,
            "p_two_sided": wilcoxon.p_two_sided,
            "wins": wins,
            "losses": losses,
            "ties": ties,
        },
        "per_query": evaluated_queries.iter().enumerate().map(|(i, q)| {
            let mut doctypes: Vec<&str> = q.labeled.grades.iter()
                .filter(|(_, g)| **g == 2)
                .map(|(p, _)| p.rsplit('.').next().unwrap_or("?"))
                .collect();
            doctypes.sort_unstable();
            doctypes.dedup();
            serde_json::json!({
                "id": q.labeled.id,
                "gold_doctype": doctypes.join("+"),
                "rr_bm25": rr_bm25[i],
                "rr_dense": rr_dense[i],
                "rr_rrf": rr_rrf[i],
                "rr_best_ecdf": best_rr_ecdf[i],
            })
        }).collect::<Vec<_>>(),
        "matrix": minmax_rr_by_alpha.iter().zip(ecdf_rr_by_alpha.iter()).map(|((a, mm), (_, ec))| {
            serde_json::json!({"alpha": a, "rr_minmax": mm, "rr_ecdf": ec})
        }).collect::<Vec<_>>(),
    });
    let report_path = root.join("target/convex_sweep_report.json");
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("report serializes"),
    )
    .expect("sweep report writes to target/");
    println!("SWEEP REPORT: {}", report_path.display());

    // 6. Test Bilateral Guard with Continuous Convex Combination
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
