//! Fusion-k sweep: why does plain RRF lose to dense alone?
//!
//! Reuses the persisted corpus + vector cache (no bulk embedding) and only
//! embeds the ~90 query strings live via plain HTTP. Compares MRR@5 across
//! fusion variants: RRF k in {60, 10, 5, 1}, dense-only, BM25-only.
//!
//! Run:
//! `CYNIC_EMBED_URL=http://192.168.0.25:8080/v1 CYNIC_EMBED_MODEL=embeddinggemma-2-Q8_0 \
//!  cargo test -p cynic-eval --test fusion_k_sweep -- --ignored --nocapture`

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    grade_chunk, reciprocal_rank, Fixture, TextPipeline, FIXTURE_SCHEMA_V1,
};
use cynic_knowledge::{
    fuse_reciprocal_ranks, load_vector_cache, sync_workspace_corpus, top_k_dense, ChunkVector,
    LexicalIndex, NormalizationStrategy, VectorCacheRecord,
};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing contracts/");
        }
    }
    dir
}

fn embed_query(base_url: &str, model: &str, text: &str) -> Vec<f32> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("http client builds");
    let body = serde_json::json!({ "input": [text], "model": model });
    let resp: serde_json::Value = client
        .post(format!("{base_url}/embeddings"))
        .json(&body)
        .send()
        .expect("embeddings request succeeds")
        .json()
        .expect("embeddings response parses");
    let vector = resp["data"][0]["embedding"]
        .as_array()
        .expect("embedding array present")
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect::<Vec<_>>();
    assert!(!vector.is_empty(), "query vector must not be empty");
    vector
}

fn config_cache_filename(root: &std::path::Path, model: &str, pipeline: &TextPipeline) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(pipeline.tag().as_bytes());
    let hash = hex::encode(hasher.finalize());
    let _ = root;
    format!("target/eval_vectors_{model}_{hash:.8}.json")
}

#[test]
#[ignore = "needs the persisted corpus/vector cache plus a live /v1/embeddings endpoint"]
fn fusion_k_sweep_is_measured() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(
        fixture.schema, FIXTURE_SCHEMA_V1,
        "fixture schema must match"
    );

    let pipeline = TextPipeline {
        max_chars: 1000,
        doc_prefix: String::new(),
        query_prefix: String::new(),
        doc_template: "{body}".into(),
    };
    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "unknown-model".into());
    let base_url = std::env::var("CYNIC_EMBED_URL").expect("CYNIC_EMBED_URL must be set");

    let vector_cache: VectorCacheRecord = load_vector_cache(
        &root.join(config_cache_filename(&root, &model, &pipeline)),
        &model,
        &pipeline.tag(),
        &base_url,
    )
    .expect("vector cache from a full run must exist");

    let (cache, _) =
        sync_workspace_corpus(&root, Some(&root.join("target/eval_corpus_cache.json")), 5);
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    let total_chunks = chunks.len();
    chunks.retain(|chunk| vector_cache.vectors.contains_key(&chunk.chunk_id));
    println!(
        "SWEEP cache coverage: {}/{} chunks ({:.1}%)",
        chunks.len(),
        total_chunks,
        chunks.len() as f64 * 100.0 / total_chunks.max(1) as f64
    );
    let vectors: Vec<ChunkVector> = chunks
        .iter()
        .map(|chunk| ChunkVector {
            chunk_id: chunk.chunk_id.clone(),
            vector: vector_cache
                .vectors
                .get(&chunk.chunk_id)
                .unwrap_or_else(|| panic!("vector missing for {}", chunk.chunk_id))
                .clone(),
        })
        .collect();
    let index = LexicalIndex::build(chunks.clone());

    let ks = [60.0, 10.0, 5.0, 1.0];
    let alphas = [0.0, 0.25, 0.5, 0.75, 1.0];
    let strategies = [
        ("minmax", NormalizationStrategy::CandidateMinMax),
        ("ecdf", NormalizationStrategy::EmpiricalCdf),
        ("absband", NormalizationStrategy::AbsoluteBand),
    ];
    let mut sums = vec![0.0; ks.len() + 2 + alphas.len() * strategies.len()];
    let mut measured = 0usize;
    for labeled in &fixture.queries {
        if labeled.trap {
            continue;
        }
        let query_text = pipeline.query_text(&labeled.query);
        let query_vector = embed_query(&base_url, &model, &query_text);
        let sparse = index.search(&labeled.query, 96);
        let dense = top_k_dense(&query_vector, &chunks, &vectors, 20);
        for (slot, k) in ks.iter().enumerate() {
            let mut fused = fuse_reciprocal_ranks(&sparse, &dense, *k);
            fused.truncate(5);
            let grades: Vec<u32> = fused
                .iter()
                .map(|hit| grade_chunk(hit.chunk, &labeled.grades))
                .collect();
            sums[slot] += reciprocal_rank(&grades);
        }
        let grade_dense: Vec<u32> = dense
            .iter()
            .take(5)
            .map(|hit| grade_chunk(hit.chunk, &labeled.grades))
            .collect();
        let grade_sparse: Vec<u32> = sparse
            .iter()
            .take(5)
            .map(|hit| grade_chunk(hit.chunk, &labeled.grades))
            .collect();
        sums[ks.len()] += reciprocal_rank(&grade_dense);
        sums[ks.len() + 1] += reciprocal_rank(&grade_sparse);
        for (ai, alpha) in alphas.iter().enumerate() {
            for (si, (sname, strategy)) in strategies.iter().enumerate() {
                let _ = sname;
                let mut fused =
                    cynic_knowledge::fuse_continuous_convex(&sparse, &dense, *alpha, *strategy);
                fused.truncate(5);
                let grades: Vec<u32> = fused
                    .iter()
                    .map(|hit| grade_chunk(hit.chunk, &labeled.grades))
                    .collect();
                sums[ks.len() + 2 + ai * strategies.len() + si] += reciprocal_rank(&grades);
            }
        }
        measured += 1;
    }
    let mut line = format!("SWEEP over {measured} queries:");
    for (slot, k) in ks.iter().enumerate() {
        line.push_str(&format!(" rrf_k={k}: {:.3}", sums[slot] / measured as f64));
    }
    line.push_str(&format!(
        " dense_only: {:.3} bm25_only: {:.3}",
        sums[ks.len()] / measured as f64,
        sums[ks.len() + 1] / measured as f64
    ));
    for (ai, alpha) in alphas.iter().enumerate() {
        for (si, (sname, _)) in strategies.iter().enumerate() {
            line.push_str(&format!(
                " convex_a={alpha}_{sname}: {:.3}",
                sums[ks.len() + 2 + ai * strategies.len() + si] / measured as f64
            ));
        }
    }
    println!("{line}");
}
