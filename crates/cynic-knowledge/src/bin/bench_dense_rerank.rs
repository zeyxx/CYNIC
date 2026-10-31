//! Dense + rerank retrieval benchmark.
//!
//! Measures MRR + nDCG@5 for:
//!   - BM25 only
//!   - BM25 + dense (RRF fusion)
//!   - BM25 + dense + rerank

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_knowledge::{KnowledgeGraph, scan_corpus_manifest, sync_workspace_corpus};
use serde::Deserialize;

const EMBEDDING_URL: &str = "http://localhost:8080/v1/embeddings";
const RERANK_URL: &str = "http://localhost:8082/v1/rerank";
const TOP_K: usize = 5;
const RRF_K: f64 = 60.0;

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

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find repo root containing contracts/");
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

fn ndcg_at_k(grades_ranked: &[u32]) -> f64 {
    let dcg: f64 = grades_ranked
        .iter()
        .enumerate()
        .map(|(i, grade)| (f64::from(1u32 << grade) - 1.0) / ((i + 2) as f64).log2())
        .sum();
    let mut ideal = grades_ranked.to_vec();
    ideal.sort_unstable_by(|a, b| b.cmp(a));
    let idcg: f64 = ideal
        .iter()
        .enumerate()
        .map(|(i, grade)| (f64::from(1u32 << grade) - 1.0) / ((i + 2) as f64).log2())
        .sum();
    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

async fn embed_text(text: &str) -> Vec<f32> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "input": text
    });
    let resp = client
        .post(EMBEDDING_URL)
        .json(&body)
        .send()
        .await
        .expect("embedding request failed");
    let text = resp.text().await.expect("read response text");
    println!("DEBUG embedding response: {}", &text[..text.len().min(200)]);
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse embedding response");
    json["data"][0]["embedding"]
        .as_array()
        .expect("embedding array")
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect()
}

#[tokio::main]
async fn main() {
    let root = repo_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(fixture.schema, "cynic.retrieval_gain_labels.v1");

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (cache, index) = cynic_knowledge::sync_workspace_corpus(&root, Some(&cache_file), 5);
    assert!(index.chunk_count() > 1000);

    println!("Loaded {} chunks", index.chunk_count());

    // Pre-compute embeddings for all chunks (sample first 500 for speed)
    println!("Computing embeddings for corpus chunks...");
    let mut chunk_embeddings: Vec<(String, Vec<f32>)> = Vec::new();
    let chunks: Vec<_> = index.chunks().iter().take(500).collect();
    for chunk in &chunks {
        let emb = embed_text(&chunk.content).await;
        chunk_embeddings.push((chunk.source_path.clone(), emb));
    }
    println!("Computed {} embeddings", chunk_embeddings.len());

    let mut bm25_mrr_sum = 0.0;
    let mut bm25_ndcg_sum = 0.0;
    let mut dense_mrr_sum = 0.0;
    let mut dense_ndcg_sum = 0.0;
    let mut rrf_mrr_sum = 0.0;
    let mut rrf_ndcg_sum = 0.0;
    let n = fixture.queries.len() as f64;

    for labeled in &fixture.queries {
        // BM25
        let bm25_hits = index.search(&labeled.query, TOP_K);
        let bm25_grades: Vec<u32> = bm25_hits
            .iter()
            .map(|hit| grade_hit(&hit.chunk.source_path, &labeled.grades))
            .collect();
        let bm25_rank = bm25_grades.iter().position(|g| *g >= 1).map(|i| i + 1);
        let bm25_rr = bm25_rank.map(|r| 1.0 / r as f64).unwrap_or(0.0);
        let bm25_ndcg = ndcg_at_k(&bm25_grades);

        // Dense (cosine similarity)
        let query_emb = embed_text(&labeled.query).await;
        let mut dense_scores: Vec<(usize, f32)> = chunk_embeddings
            .iter()
            .enumerate()
            .map(|(i, (_, emb))| (i, cosine_similarity(&query_emb, emb)))
            .collect();
        dense_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let dense_paths: Vec<String> = dense_scores
            .iter()
            .take(TOP_K)
            .map(|(i, _)| chunk_embeddings[*i].0.clone())
            .collect();
        let dense_grades: Vec<u32> = dense_paths
            .iter()
            .map(|p| grade_hit(p, &labeled.grades))
            .collect();
        let dense_rank = dense_grades.iter().position(|g| *g >= 1).map(|i| i + 1);
        let dense_rr = dense_rank.map(|r| 1.0 / r as f64).unwrap_or(0.0);
        let dense_ndcg = ndcg_at_k(&dense_grades);

        // RRF fusion (BM25 + dense)
        let mut rrf_scores: HashMap<String, f64> = HashMap::new();
        for (rank, hit) in bm25_hits.iter().enumerate() {
            *rrf_scores.entry(hit.chunk.source_path.clone()).or_insert(0.0) +=
                1.0 / (RRF_K + rank as f64 + 1.0);
        }
        for (rank, path) in dense_paths.iter().enumerate() {
            *rrf_scores.entry(path.clone()).or_insert(0.0) +=
                1.0 / (RRF_K + rank as f64 + 1.0);
        }
        let mut rrf_ranked: Vec<(String, f64)> = rrf_scores.into_iter().collect();
        rrf_ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let rrf_paths: Vec<String> = rrf_ranked.iter().take(TOP_K).map(|(p, _)| p.clone()).collect();
        let rrf_grades: Vec<u32> = rrf_paths.iter().map(|p| grade_hit(p, &labeled.grades)).collect();
        let rrf_rank = rrf_grades.iter().position(|g| *g >= 1).map(|i| i + 1);
        let rrf_rr = rrf_rank.map(|r| 1.0 / r as f64).unwrap_or(0.0);
        let rrf_ndcg = ndcg_at_k(&rrf_grades);

        bm25_mrr_sum += bm25_rr;
        bm25_ndcg_sum += bm25_ndcg;
        dense_mrr_sum += dense_rr;
        dense_ndcg_sum += dense_ndcg;
        rrf_mrr_sum += rrf_rr;
        rrf_ndcg_sum += rrf_ndcg;
    }

    println!(
        "AVERAGE: BM25 MRR={:.3} nDCG={:.3} | DENSE MRR={:.3} nDCG={:.3} | RRF MRR={:.3} nDCG={:.3}",
        bm25_mrr_sum / n,
        bm25_ndcg_sum / n,
        dense_mrr_sum / n,
        dense_ndcg_sum / n,
        rrf_mrr_sum / n,
        rrf_ndcg_sum / n
    );
}
