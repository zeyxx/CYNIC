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
use cynic_knowledge::{
    ChunkEmbedder, VectorCacheRecord, load_vector_cache, save_vector_cache, sync_workspace_corpus,
    vectorize_chunks_cached,
};
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

fn truncate_front(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        text
    } else {
        let mut end = max_chars;
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
    // Graduated calibration: chunk ids are content hashes, so taking the
    // first N is a uniform random sample, not a directory-biased prefix.
    // Sweep cheap (100, 1000), confirm full (0 = all).
    let subset: usize = std::env::var("CYNIC_EMBED_SUBSET")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(0);
    if subset > 0 && subset < chunks.len() {
        chunks.truncate(subset);
    }
    println!(
        "NEURAL_CORPUS chunks={} (subset={})",
        chunks.len(),
        if subset > 0 {
            subset.to_string()
        } else {
            "all".into()
        }
    );
    let corpus_start = std::time::Instant::now();
    let last_report = std::cell::Cell::new(0usize);
    // Truncation is deterministic, so vectors of truncated texts stay valid
    // under the full chunk's content-addressed id across runs. Instruction
    // prefixes (EmbeddingGemma retrieval protocol) are part of the text
    // pipeline: any prefix change must invalidate the cache, hence the tag.
    // Titles follow the card (`title: {filename} | text: {code}`); the MRL
    // dimension applies after the cache, so one 768d cache serves every dim.
    let doc_prefix = std::env::var("CYNIC_EMBED_DOC_PREFIX").unwrap_or_default();
    let query_prefix = std::env::var("CYNIC_EMBED_QUERY_PREFIX").unwrap_or_default();
    let max_chars: usize = std::env::var("CYNIC_EMBED_MAX_CHARS")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .filter(|max| (256..=8000).contains(max))
        .unwrap_or(MAX_EMBED_CHARS);
    let trunc_dim: usize = std::env::var("CYNIC_EMBED_TRUNC_DIM")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .filter(|dim| (32..=4096).contains(dim))
        .unwrap_or(0);
    // Model-agnostic document template: `{body}` is the truncated chunk,
    // `{filename}` its source file name. Default is the body alone; a model
    // card protocol is expressed as data (e.g. Gemma's
    // `"title: {filename} | text: {body}"`), never as framework behavior.
    let doc_template =
        std::env::var("CYNIC_EMBED_DOC_TEMPLATE").unwrap_or_else(|_| "{body}".into());
    let pipeline =
        format!("trunc{max_chars}/doc={doc_prefix}/query={query_prefix}/template={doc_template}");
    struct Truncating<'a> {
        bridge: &'a SyncEmbeddingBridge<EmbeddingBackend>,
        last_report: &'a std::cell::Cell<usize>,
        corpus_start: std::time::Instant,
        doc_prefix: &'a str,
        max_chars: usize,
        doc_template: &'a str,
    }
    impl ChunkEmbedder for Truncating<'_> {
        fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            self.bridge.embed_batch_reported(texts, &|_, _| {})
        }

        fn embed_chunks(
            &self,
            chunks: &[cynic_knowledge::DocumentChunk],
        ) -> Result<Vec<Vec<f32>>, String> {
            let owned: Vec<String> = chunks
                .iter()
                .map(|chunk| {
                    let body = truncate_front(&chunk.content, self.max_chars);
                    let filename = chunk.source_path.rsplit('/').next().unwrap_or("none");
                    format!(
                        "{}{}",
                        self.doc_prefix,
                        self.doc_template
                            .replace("{filename}", filename)
                            .replace("{body}", body)
                    )
                })
                .collect();
            let titled: Vec<&str> = owned.iter().map(String::as_str).collect();
            self.bridge.embed_batch_reported(&titled, &|done, total| {
                if done >= self.last_report.get() + 320 || done == total {
                    self.last_report.set(done);
                    println!(
                        "EMBED {done}/{total} chunks ({}%) elapsed={:.0}s",
                        done * 100 / total,
                        self.corpus_start.elapsed().as_secs_f32()
                    );
                }
            })
        }
    }
    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "unknown-model".into());
    let vector_cache_file = root.join(format!("target/eval_vectors_{model}.json"));
    let mut vector_cache = load_vector_cache(&vector_cache_file, &model, &pipeline)
        .unwrap_or_else(|| VectorCacheRecord::new(&model, &pipeline));
    let adapter = Truncating {
        bridge: &bridge,
        last_report: &last_report,
        corpus_start,
        doc_prefix: &doc_prefix,
        max_chars,
        doc_template: &doc_template,
    };
    // Leading-dimension truncation applies after the cache (cosine divides by
    // norms, so no renormalization is needed for ranking): one full-dim
    // cache serves every truncation experiment. Queries and corpus share it.
    let (mut vectors, reused, embedded) =
        vectorize_chunks_cached(&adapter, &chunks, &mut vector_cache, &|_, _| {})
            .expect("corpus vectors");
    save_vector_cache(&vector_cache_file, &vector_cache).expect("persist vector cache");
    if trunc_dim > 0 {
        for pair in &mut vectors {
            pair.vector.truncate(trunc_dim);
        }
    }
    println!(
        "EMBED done: {} vectors ({} reused, {} embedded) in {:.0}s",
        vectors.len(),
        reused,
        embedded,
        corpus_start.elapsed().as_secs_f32()
    );

    println!(
        "{:<24} {:>8} {:>8} {:>8}  top1-hybrid",
        "query", "MRR-bm25", "MRR-dens", "MRR-hybr"
    );
    let mut sum_bm25 = 0.0;
    let mut sum_dense = 0.0;
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
        let query_text = format!(
            "{query_prefix}{}",
            truncate_front(&labeled.query, max_chars)
        );
        let query_vector = bridge
            .embed_batch(std::slice::from_ref(&query_text.as_str()))
            .expect("query vector");
        let mut query_vector = query_vector;
        if trunc_dim > 0 {
            for vector in &mut query_vector {
                vector.truncate(trunc_dim);
            }
        }
        let hybrid_hits =
            index.search_hybrid(&labeled.query, &query_vector[0], &vectors, TOP_K, DENSE_K);
        assert!(
            !hybrid_hits.is_empty(),
            "[{}] hybrid must retrieve at least one hit",
            labeled.id
        );
        // Isolated dense ranking: same vectors, no BM25, no fusion — the
        // column that tells whether the dense side or the fusion is at fault.
        let dense_hits = cynic_knowledge::top_k_dense(&query_vector[0], &chunks, &vectors, TOP_K);

        let bm25_rr = reciprocal_rank(
            &bm25_hits
                .iter()
                .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
                .collect::<Vec<_>>(),
        );
        let dense_rr = reciprocal_rank(
            &dense_hits
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
        sum_dense += dense_rr;
        sum_hybrid += hybrid_rr;
        measured += 1;
        let top1 = hybrid_hits
            .first()
            .map(|hit| hit.chunk.source_path.as_str())
            .unwrap_or("<none>");
        println!(
            "{:<24} {:>8.3} {:>8.3} {:>8.3}  {top1}",
            labeled.id, bm25_rr, dense_rr, hybrid_rr
        );
    }

    assert!(measured > 0, "fixture must contain non-trap queries");
    let mean_bm25 = sum_bm25 / measured as f64;
    let mean_dense = sum_dense / measured as f64;
    let mean_hybrid = sum_hybrid / measured as f64;
    println!(
        "NEURAL_MEAN MRR over {measured} queries: bm25={mean_bm25:.3} dense={mean_dense:.3} hybrid={mean_hybrid:.3} delta={:.3}",
        mean_hybrid - mean_bm25
    );
    let report = serde_json::json!({
        "schema": "cynic.neural_hybrid_gain.v1",
        "measured_queries": measured,
        "mean_mrr_bm25": mean_bm25,
        "mean_mrr_dense": mean_dense,
        "mean_mrr_hybrid": mean_hybrid,
        "delta_hybrid_minus_bm25": mean_hybrid - mean_bm25,
    });
    let _ = std::fs::write(
        root.join("target/neural_hybrid_gain.json"),
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
}
