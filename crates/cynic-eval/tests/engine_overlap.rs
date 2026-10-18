//! Cross-engine ranking overlap: how much does the 2.5e-3 numeric drift
//! disturb retrieval order?
//!
//! Corpus vectors come from the persisted compute-01 cache; cortex corpus
//! vectors are embedded live (fast engine, ~2 min). Each non-trap query is
//! embedded on both engines, top-20 dense lists are compared (Jaccard +
//! mean rank displacement of common chunks + top-1 agreement).
//!
//! Run:
//! `CYNIC_EMBED_URL=http://192.168.0.25:8080/v1 CYNIC_EMBED_MODEL=embeddinggemma-2-Q8_0 \
//!  CYNIC_EMBED_URL_B=http://127.0.0.1:8081/v1 \
//!  cargo test -p cynic-eval --test engine_overlap -- --ignored --nocapture`

use std::collections::HashSet;
use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{Fixture, TextPipeline, FIXTURE_SCHEMA_V1};
use cynic_knowledge::{
    load_vector_cache, sync_workspace_corpus, top_k_dense, ChunkVector, LexicalIndex,
    VectorCacheRecord,
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

fn embed_batch(base_url: &str, model: &str, texts: &[String]) -> Vec<Vec<f32>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .expect("http client builds");
    let mut out = Vec::with_capacity(texts.len());
    for chunk in texts.chunks(64) {
        let body = serde_json::json!({ "input": chunk, "model": model });
        let resp: serde_json::Value = client
            .post(format!("{base_url}/embeddings"))
            .json(&body)
            .send()
            .expect("embeddings request succeeds")
            .json()
            .expect("embeddings response parses");
        for entry in resp["data"].as_array().expect("data array") {
            out.push(
                entry["embedding"]
                    .as_array()
                    .expect("embedding array")
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect(),
            );
        }
    }
    assert_eq!(out.len(), texts.len(), "one vector per input text");
    out
}

#[test]
#[ignore = "needs both live /v1/embeddings endpoints plus the compute-01 vector cache"]
fn engine_overlap_is_measured() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(fixture.schema, FIXTURE_SCHEMA_V1);

    let pipeline = TextPipeline {
        max_chars: 1000,
        doc_prefix: String::new(),
        query_prefix: String::new(),
        doc_template: "{body}".into(),
    };
    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "unknown-model".into());
    let url_a = std::env::var("CYNIC_EMBED_URL").expect("CYNIC_EMBED_URL must be set");
    let url_b = std::env::var("CYNIC_EMBED_URL_B").expect("CYNIC_EMBED_URL_B must be set");
    assert_ne!(url_a, url_b, "the two engines must differ");

    let vector_cache: VectorCacheRecord = match load_vector_cache(
        &root.join("target/eval_vectors_compute01.json"),
        &model,
        &pipeline.tag(),
        &url_a,
    ) {
        Some(cache) => cache,
        None => {
            // Fall back to the hashed cache filename used by the full bench run.
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(pipeline.tag().as_bytes());
            let hash = hex::encode(hasher.finalize());
            load_vector_cache(
                &root.join(format!("target/eval_vectors_{model}_{hash:.8}.json")),
                &model,
                &pipeline.tag(),
                &url_a,
            )
            .expect("compute-01 vector cache must exist")
        }
    };

    let (cache, _) =
        sync_workspace_corpus(&root, Some(&root.join("target/eval_corpus_cache.json")), 5);
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    chunks.retain(|c| vector_cache.vectors.contains_key(&c.chunk_id));
    println!(
        "OVERLAP corpus: {} chunks with compute-01 vectors",
        chunks.len()
    );

    // Cortex corpus vectors, embedded live in batches.
    let doc_texts: Vec<String> = chunks.iter().map(|c| pipeline.doc_text(c)).collect();
    let cortex_vecs = embed_batch(&url_b, &model, &doc_texts);
    let vecs_a: Vec<ChunkVector> = chunks
        .iter()
        .map(|c| ChunkVector {
            chunk_id: c.chunk_id.clone(),
            vector: vector_cache.vectors[&c.chunk_id].clone(),
        })
        .collect();
    let vecs_b: Vec<ChunkVector> = chunks
        .iter()
        .zip(cortex_vecs.iter())
        .map(|(c, v)| ChunkVector {
            chunk_id: c.chunk_id.clone(),
            vector: v.clone(),
        })
        .collect();
    let _index = LexicalIndex::build(chunks.clone());

    let mut jaccard_sum = 0.0;
    let mut disp_sum = 0.0;
    let mut disp_n = 0usize;
    let mut top1_agree = 0usize;
    let mut measured = 0usize;
    for labeled in &fixture.queries {
        if labeled.trap {
            continue;
        }
        let query_text = pipeline.query_text(&labeled.query);
        let qa = embed_batch(&url_a, &model, &[query_text.clone()])
            .pop()
            .unwrap();
        let qb = embed_batch(&url_b, &model, &[query_text]).pop().unwrap();
        let top_a = top_k_dense(&qa, &chunks, &vecs_a, 20);
        let top_b = top_k_dense(&qb, &chunks, &vecs_b, 20);
        let set_a: HashSet<&str> = top_a.iter().map(|h| h.chunk.chunk_id.as_str()).collect();
        let set_b: HashSet<&str> = top_b.iter().map(|h| h.chunk.chunk_id.as_str()).collect();
        let inter = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();
        jaccard_sum += inter as f64 / union.max(1) as f64;
        let rank_b: std::collections::HashMap<&str, usize> = top_b
            .iter()
            .enumerate()
            .map(|(i, h)| (h.chunk.chunk_id.as_str(), i))
            .collect();
        for (i, h) in top_a.iter().enumerate() {
            if let Some(&j) = rank_b.get(h.chunk.chunk_id.as_str()) {
                disp_sum += (i as i64 - j as i64).unsigned_abs() as f64;
                disp_n += 1;
            }
        }
        if top_a.first().map(|h| h.chunk.chunk_id.as_str())
            == top_b.first().map(|h| h.chunk.chunk_id.as_str())
        {
            top1_agree += 1;
        }
        measured += 1;
    }
    println!(
        "OVERLAP over {measured} queries: top20_jaccard {:.3} mean_rank_displacement {:.2} top1_agreement {:.3}",
        jaccard_sum / measured.max(1) as f64,
        disp_sum / disp_n.max(1) as f64,
        top1_agree as f64 / measured.max(1) as f64
    );
}
