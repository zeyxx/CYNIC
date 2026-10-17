//! Total-miss diagnosis for the 6 queries no arm answers (Chapter 5 follow-up).
//!
//! Reuses the trunc2500 vector cache (zero re-embedding cost, 6 query
//! vectors only) and reports, per miss query, gold rank at depth 100 in
//! each arm plus the top-5 paths of each arm.
//!
//! Run with:
//! ```bash
//! CYNIC_EMBED_URL="http://127.0.0.1:8081/v1#slots=2,http://192.168.0.25:8080/v1#slots=2" \
//! CYNIC_EMBED_MODEL="multi-endpoint" \
//! cargo test -p cynic-inference --test miss_diagnosis -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, FIXTURE_SCHEMA_V1, Fixture, TemplatedEmbedder, TextPipeline, VectorCacheRecord,
    grade_chunk, load_vector_cache,
};
use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;
use cynic_knowledge::{ChunkEmbedder, sync_workspace_corpus, top_k_dense, vectorize_chunks_cached};

const MISS_IDS: &[&str] = &[
    "RQC1_CONSENSUS",
    "RQC4_PARAPHRASE",
    "RQ19_FOUNDATION_INVARIANTS",
    "RQ20_CLAIMS_LEASE",
    "RQ26_MACHINE_IDENTITY",
    "RQ28_EVIDENCE_RECEIPT",
];

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
fn diagnose_total_miss_queries() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(fixture.schema, FIXTURE_SCHEMA_V1);

    let backend = EmbeddingBackend::from_env().expect("CYNIC_EMBED_URL must be set");
    let bridge = SyncEmbeddingBridge::probe(backend, "CYNIC miss diagnosis")
        .expect("embedding probe must succeed");

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));

    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "multi-endpoint".into());
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
        .unwrap_or_else(|| VectorCacheRecord::new(&model, &pipeline.tag()));
    let templated = TemplatedEmbedder::new(&bridge, &pipeline, config.embed_lot, &|_, _| {});
    let (vectors, reused, embedded) =
        vectorize_chunks_cached(&templated, &chunks, &mut vector_cache, &|_, _| {})
            .expect("vectorization must succeed");
    println!(
        "CORPUS: {} chunks, vectors reused={} embedded={}",
        chunks.len(),
        reused,
        embedded
    );

    let diag_ids: Vec<String> = match std::env::var("CYNIC_DIAG_IDS") {
        Ok(list) => list.split(',').map(|s| s.trim().to_string()).collect(),
        Err(_) => MISS_IDS.iter().map(|s| s.to_string()).collect(),
    };
    for miss_id in diag_ids {
        let labeled = fixture
            .queries
            .iter()
            .find(|q| q.id == miss_id)
            .expect("miss id must exist in fixture");
        println!("\n=== {}: {:?} ===", labeled.id, labeled.query);

        let gold_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| grade_chunk(c, &labeled.grades) >= 1)
            .collect();
        println!("gold chunks in corpus: {}", gold_chunks.len());
        for g in gold_chunks.iter().take(5) {
            println!(
                "  gold: {}:{}-{} ({} chars)",
                g.source_path,
                g.line_start,
                g.line_end,
                g.content.len()
            );
        }
        if gold_chunks.is_empty() {
            println!("  NO GOLD CHUNK INDEXED: grade path never surfaces -> fixture/corpus gap");
            continue;
        }

        let sparse100 = index.search(&labeled.query, 100);
        let gold_rank_bm25 = sparse100
            .iter()
            .position(|h| grade_chunk(h.chunk, &labeled.grades) >= 1)
            .map(|i| i + 1);
        println!("BM25 gold rank @100: {:?}", gold_rank_bm25);
        for (i, h) in sparse100.iter().take(5).enumerate() {
            println!(
                "  bm25 #{:<3} score={:8.2} {}:{}-{}",
                i + 1,
                h.score,
                h.chunk.source_path,
                h.chunk.line_start,
                h.chunk.line_end
            );
        }

        let query_text = pipeline.query_text(&labeled.query);
        let query_vector = &bridge
            .embed_batch(&[query_text.as_str()])
            .expect("embed query")[0];
        let dense100 = top_k_dense(query_vector, &chunks, &vectors, 100);
        let gold_rank_dense = dense100
            .iter()
            .position(|h| grade_chunk(h.chunk, &labeled.grades) >= 1)
            .map(|i| i + 1);
        println!("DENSE gold rank @100: {:?}", gold_rank_dense);
        for (i, h) in dense100.iter().take(5).enumerate() {
            println!(
                "  dense #{:<3} sim={:.4} {}:{}-{}",
                i + 1,
                h.similarity,
                h.chunk.source_path,
                h.chunk.line_start,
                h.chunk.line_end
            );
        }
    }
}
