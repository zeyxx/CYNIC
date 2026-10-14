//! Graded retrieval benchmark: BM25-only vs dense-only vs hybrid RRF.
//!
//! Pure orchestration over injected ports — the caller owns the environment
//! (config values), the embedding backend (`&dyn ChunkEmbedder`), the vector
//! cache record (load/save), and all printing. This module never reads env,
//! never touches the network, and never writes stdout: it computes and
//! returns. The live test in `cynic-inference` and the future
//! `cynic know eval` command are both thin entries over this API.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use cynic_knowledge::{
    ChunkEmbedder, ChunkVector, DocumentChunk, sync_workspace_corpus, top_k_dense,
    vectorize_chunks_cached,
};
pub use cynic_knowledge::{VectorCacheRecord, load_vector_cache, save_vector_cache};

pub const FIXTURE_SCHEMA_V1: &str = "cynic.retrieval_gain_labels.v1";
pub const REPORT_SCHEMA_V1: &str = "cynic.neural_hybrid_gain.v1";

/// Model-agnostic text pipeline: every byte that reaches the embedder is
/// described here, so any change invalidates cached vectors via the tag.
/// Model-card protocols are data in these fields, never framework behavior.
#[derive(Debug, Clone)]
pub struct TextPipeline {
    pub max_chars: usize,
    pub doc_prefix: String,
    pub query_prefix: String,
    pub doc_template: String,
}

impl TextPipeline {
    pub fn tag(&self) -> String {
        format!(
            "trunc{}/doc={}/query={}/template={}",
            self.max_chars, self.doc_prefix, self.query_prefix, self.doc_template
        )
    }

    /// Render one corpus chunk for the embedder. `{body}` is the truncated
    /// content, `{filename}` its source file name.
    pub fn doc_text(&self, chunk: &DocumentChunk) -> String {
        let body = truncate_front(&chunk.content, self.max_chars);
        let filename = chunk.source_path.rsplit('/').next().unwrap_or("none");
        format!(
            "{}{}",
            self.doc_prefix,
            self.doc_template
                .replace("{filename}", filename)
                .replace("{body}", body)
        )
    }

    pub fn query_text(&self, query: &str) -> String {
        format!(
            "{}{}",
            self.query_prefix,
            truncate_front(query, self.max_chars)
        )
    }
}

/// Full bench configuration: plain data, built by the entry from env/argv.
#[derive(Debug, Clone)]
pub struct BenchConfig {
    /// 0 means the whole corpus; otherwise first-N by content-hash order,
    /// which is a uniform random sample, not a directory-biased prefix.
    pub subset: usize,
    /// Leading-dimension truncation applied after the cache (0 = full dim).
    /// Cosine divides by norms, so no renormalization is needed for ranking.
    pub trunc_dim: usize,
    pub top_k: usize,
    pub dense_k: usize,
    /// Guarded-fusion threshold: relative BM25 top1-top2 margin at or above
    /// which the sparse ranking returns alone. `None` = plain RRF.
    pub guard_gap: Option<f64>,
    /// Internal embed lots for progress granularity (transport batching
    /// stays a backend concern).
    pub embed_lot: usize,
    pub pipeline: TextPipeline,
    pub model: String,
}

impl BenchConfig {
    /// Cache file scoped by model AND text pipeline: any pipeline change
    /// lands in a fresh file instead of silently reusing stale vectors.
    pub fn vector_cache_filename(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.pipeline.tag().as_bytes());
        let hash = hex::encode(hasher.finalize());
        format!("target/eval_vectors_{}_{:.8}.json", self.model, hash)
    }
}

/// `&dyn ChunkEmbedder` adapter that renders the text pipeline around a
/// silent batch backend, with throttled progress per internal lot.
pub struct TemplatedEmbedder<'a> {
    inner: &'a dyn ChunkEmbedder,
    pipeline: &'a TextPipeline,
    lot: usize,
    done: std::cell::Cell<usize>,
    report: &'a dyn Fn(usize, usize),
}

impl<'a> TemplatedEmbedder<'a> {
    pub fn new(
        inner: &'a dyn ChunkEmbedder,
        pipeline: &'a TextPipeline,
        lot: usize,
        report: &'a dyn Fn(usize, usize),
    ) -> Self {
        Self {
            inner,
            pipeline,
            lot: lot.max(1),
            done: std::cell::Cell::new(0),
            report,
        }
    }
}

impl ChunkEmbedder for TemplatedEmbedder<'_> {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        self.inner.embed_batch(texts)
    }

    fn embed_chunks(&self, chunks: &[DocumentChunk]) -> Result<Vec<Vec<f32>>, String> {
        let rendered: Vec<String> = chunks
            .iter()
            .map(|chunk| self.pipeline.doc_text(chunk))
            .collect();
        let mut out = Vec::new();
        let total = rendered.len();
        for lot in rendered.chunks(self.lot) {
            let refs: Vec<&str> = lot.iter().map(String::as_str).collect();
            out.extend(self.inner.embed_batch(&refs)?);
            self.done.set(self.done.get() + lot.len());
            (self.report)(self.done.get(), total);
        }
        Ok(out)
    }
}

#[derive(Debug, Deserialize)]
pub struct LabeledQuery {
    pub id: String,
    pub query: String,
    #[serde(default)]
    pub trap: bool,
    #[serde(default)]
    pub grades: HashMap<String, u32>,
}

#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub schema: String,
    pub queries: Vec<LabeledQuery>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryScores {
    pub id: String,
    pub mrr_bm25: f64,
    pub mrr_dense: f64,
    pub mrr_hybrid: f64,
    pub top1_hybrid: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchReport {
    pub schema: String,
    pub measured_queries: usize,
    pub corpus_chunks: usize,
    pub vectors_reused: usize,
    pub vectors_embedded: usize,
    pub embed_secs: f64,
    pub mean_mrr_bm25: f64,
    pub mean_mrr_dense: f64,
    pub mean_mrr_hybrid: f64,
    pub rows: Vec<QueryScores>,
}

impl BenchReport {
    pub fn delta_hybrid_minus_bm25(&self) -> f64 {
        self.mean_mrr_hybrid - self.mean_mrr_bm25
    }

    pub fn table(&self) -> String {
        let mut text = format!(
            "{:<24} {:>8} {:>8} {:>8}  top1-hybrid\n",
            "query", "MRR-bm25", "MRR-dens", "MRR-hybr"
        );
        for row in &self.rows {
            text.push_str(&format!(
                "{:<24} {:>8.3} {:>8.3} {:>8.3}  {}\n",
                row.id, row.mrr_bm25, row.mrr_dense, row.mrr_hybrid, row.top1_hybrid
            ));
        }
        text.push_str(&format!(
            "NEURAL_MEAN MRR over {} queries: bm25={:.3} dense={:.3} hybrid={:.3} delta={:.3}\n",
            self.measured_queries,
            self.mean_mrr_bm25,
            self.mean_mrr_dense,
            self.mean_mrr_hybrid,
            self.delta_hybrid_minus_bm25()
        ));
        text
    }
}

/// Front-truncate to `max_chars` on a char boundary; short inputs pass
/// through untouched so the cache stays valid across limit raises.
pub fn truncate_front(text: &str, max_chars: usize) -> &str {
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

/// Grade of one retrieved path: best matching labeled substring, else 0.
pub fn grade_hit(path: &str, grades: &HashMap<String, u32>) -> u32 {
    grades
        .iter()
        .filter(|(labeled, _)| path.contains(labeled.as_str()))
        .map(|(_, grade)| *grade)
        .max()
        .unwrap_or(0)
}

/// Reciprocal rank of the first relevant grade in rank order.
pub fn reciprocal_rank(grades_ranked: &[u32]) -> f64 {
    grades_ranked
        .iter()
        .position(|grade| *grade >= 1)
        .map(|i| 1.0 / (i + 1) as f64)
        .unwrap_or(0.0)
}

fn truncate_dim(vectors: &mut [ChunkVector], dim: usize) {
    if dim > 0 {
        for pair in vectors.iter_mut() {
            pair.vector.truncate(dim);
        }
    }
}

/// Run the full bench: corpus sync, cached vectorize, per-query triple
/// measurement. Fails closed (Err) on empty corpus or backend errors —
/// a bench that measures nothing must say so, not print zeros.
pub fn run_bench(
    root: &Path,
    corpus_cache_file: &Path,
    fixture: &Fixture,
    config: &BenchConfig,
    backend: &dyn ChunkEmbedder,
    vector_cache: &mut VectorCacheRecord,
    report: &dyn Fn(usize, usize),
) -> Result<BenchReport, String> {
    assert_eq!(
        fixture.schema, FIXTURE_SCHEMA_V1,
        "fixture schema marker must match"
    );
    let (cache, index) = sync_workspace_corpus(root, Some(corpus_cache_file), 5);
    if index.chunk_count() <= 1000 {
        return Err(format!(
            "corpus too small to measure: {} chunks",
            index.chunk_count()
        ));
    }
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    if config.subset > 0 && config.subset < chunks.len() {
        chunks.truncate(config.subset);
    }

    let templated = TemplatedEmbedder::new(backend, &config.pipeline, config.embed_lot, report);
    let embed_start = std::time::Instant::now();
    let (mut vectors, reused, embedded) =
        vectorize_chunks_cached(&templated, &chunks, vector_cache, &|_, _| {})?;
    let embed_secs = embed_start.elapsed().as_secs_f64();
    truncate_dim(&mut vectors, config.trunc_dim);

    let mut rows = Vec::new();
    let (mut sum_bm25, mut sum_dense, mut sum_hybrid) = (0.0, 0.0, 0.0);
    for labeled in &fixture.queries {
        if labeled.trap {
            continue;
        }
        let bm25_hits = index.search(&labeled.query, config.top_k);
        if bm25_hits.is_empty() {
            return Err(format!("[{}] valid query retrieved nothing", labeled.id));
        }
        let query_text = config.pipeline.query_text(&labeled.query);
        let mut query_vectors = backend.embed_batch(&[query_text.as_str()])?;
        if query_vectors.len() != 1 {
            return Err(format!("[{}] backend returned no query vector", labeled.id));
        }
        if config.trunc_dim > 0 {
            query_vectors[0].truncate(config.trunc_dim);
        }
        // Isolated dense ranking at fusion depth (DENSE_K, not TOP_K): the
        // fusion consumes the dense top-20, so grading only the top-5 would
        // understate the dense contribution (seen live: dense top-5 misses
        // that hybrid still promotes to rank 1).
        let dense_hits = top_k_dense(&query_vectors[0], &chunks, &vectors, config.dense_k);
        let hybrid_hits = index.search_hybrid_guarded(
            &labeled.query,
            &query_vectors[0],
            &vectors,
            config.top_k,
            config.dense_k,
            config.guard_gap,
        );
        if hybrid_hits.is_empty() {
            return Err(format!("[{}] hybrid retrieved nothing", labeled.id));
        }
        let rr = |grades: Vec<u32>| reciprocal_rank(&grades);
        let bm25_rr = rr(bm25_hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect());
        let dense_rr = rr(dense_hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect());
        let hybrid_rr = rr(hybrid_hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect());
        sum_bm25 += bm25_rr;
        sum_dense += dense_rr;
        sum_hybrid += hybrid_rr;
        rows.push(QueryScores {
            id: labeled.id.clone(),
            mrr_bm25: bm25_rr,
            mrr_dense: dense_rr,
            mrr_hybrid: hybrid_rr,
            top1_hybrid: hybrid_hits
                .first()
                .map(|hit| hit.chunk.source_path.clone())
                .unwrap_or_else(|| "<none>".into()),
        });
    }
    if rows.is_empty() {
        return Err("fixture contains no non-trap queries".into());
    }
    let measured = rows.len() as f64;
    Ok(BenchReport {
        schema: REPORT_SCHEMA_V1.to_string(),
        measured_queries: rows.len(),
        corpus_chunks: chunks.len(),
        vectors_reused: reused,
        vectors_embedded: embedded,
        embed_secs,
        mean_mrr_bm25: sum_bm25 / measured,
        mean_mrr_dense: sum_dense / measured,
        mean_mrr_hybrid: sum_hybrid / measured,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_front_respects_char_boundaries() {
        assert_eq!(truncate_front("abcdef", 10), "abcdef");
        assert_eq!(truncate_front("abcdef", 4), "abcd");
        // 'é' is 2 bytes: max 1 must fall back to the empty prefix, never split.
        assert_eq!(truncate_front("éa", 1), "");
        assert_eq!(truncate_front("éa", 2), "é");
    }

    #[test]
    fn pipeline_tag_changes_on_any_field() {
        let base = TextPipeline {
            max_chars: 1000,
            doc_prefix: String::new(),
            query_prefix: String::new(),
            doc_template: "{body}".into(),
        };
        let tag = base.tag();
        let mut varied = base.clone();
        varied.max_chars = 2000;
        assert_ne!(varied.tag(), tag);
        let mut varied = base.clone();
        varied.doc_prefix = "task: x | ".into();
        assert_ne!(varied.tag(), tag);
        let mut varied = base.clone();
        varied.doc_template = "title: {filename} | text: {body}".into();
        assert_ne!(varied.tag(), tag);
    }

    #[test]
    fn doc_text_applies_template_and_prefix() {
        let pipeline = TextPipeline {
            max_chars: 100,
            doc_prefix: "task: search result | ".into(),
            query_prefix: String::new(),
            doc_template: "title: {filename} | text: {body}".into(),
        };
        let chunk = DocumentChunk::new(
            "contracts/example.json".into(),
            1,
            5,
            vec!["# T".into()],
            "some content".into(),
            cynic_knowledge::AuthorityTier::L1Specification,
        );
        assert_eq!(
            pipeline.doc_text(&chunk),
            "task: search result | title: example.json | text: some content"
        );
        assert_eq!(pipeline.query_text("why hybrid"), "why hybrid");
    }

    #[test]
    fn reciprocal_rank_positions_and_grades() {
        assert_eq!(reciprocal_rank(&[]), 0.0);
        assert_eq!(reciprocal_rank(&[0, 0]), 0.0);
        assert_eq!(reciprocal_rank(&[2, 0]), 1.0);
        assert_eq!(reciprocal_rank(&[0, 1]), 0.5);
        let grades = HashMap::from([("a/b.md".to_string(), 2), ("c.md".to_string(), 1)]);
        assert_eq!(grade_hit("x/a/b.md", &grades), 2);
        assert_eq!(grade_hit("c.md", &grades), 1);
        assert_eq!(grade_hit("other.md", &grades), 0);
    }

    #[test]
    fn cache_filename_scopes_pipeline() {
        let config = BenchConfig {
            subset: 0,
            trunc_dim: 0,
            top_k: 5,
            dense_k: 20,
            guard_gap: None,
            embed_lot: 32,
            pipeline: TextPipeline {
                max_chars: 1000,
                doc_prefix: String::new(),
                query_prefix: String::new(),
                doc_template: "{body}".into(),
            },
            model: "model-a".into(),
        };
        let plain = config.vector_cache_filename();
        let mut other = config.clone();
        other.pipeline.query_prefix = "task: x | ".into();
        assert_ne!(other.vector_cache_filename(), plain);
        assert!(plain.contains("model-a"));
    }
}
