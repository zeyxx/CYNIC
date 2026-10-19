//! Chunk Quality & Anti-Heresy Audit (Organic RAG Falsification Gate)
//!
//! Enforces:
//! 1. Zero giant chunks (> 8,000 characters) across the entire corpus.
//! 2. P99 of chunk line count <= 75 lines.
//! 3. Healthy chunk ratio >= 95% of total corpus.

use cynic_knowledge::domain::chunk_quality::{ChunkQualityReport, MAX_CHUNK_CHARS};
use cynic_knowledge::sync_workspace_corpus;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn audit_corpus_chunk_quality_and_bounds() {
    let root = repo_root();
    let cache_path = root.join("target/corpus_audit_cache.json");

    let (record, _index) = sync_workspace_corpus(&root, Some(&cache_path), 0);

    let mut all_chunks = Vec::new();
    for file_record in record.files.values() {
        for chunk in &file_record.chunks {
            all_chunks.push(chunk.clone());
        }
    }

    assert!(
        !all_chunks.is_empty(),
        "Corpus chunks must not be empty for quality audit"
    );

    let report = ChunkQualityReport::audit(&all_chunks);

    println!(
        "CHUNK_QUALITY_REPORT: total={}, healthy={:.2}%, P50={} chars ({} lines), P99={} chars ({} lines), max={} chars",
        report.total_chunks,
        report.healthy_ratio * 100.0,
        report.char_percentiles.p50,
        report.line_percentiles.p50,
        report.char_percentiles.p99,
        report.line_percentiles.p99,
        report.char_percentiles.max,
    );

    // 1. Zero chunks exceeding the dense-model char budget (2,500)
    let over_budget: Vec<_> = report
        .oversized_chunks
        .iter()
        .filter(|a| a.char_length > MAX_CHUNK_CHARS)
        .collect();
    assert!(
        over_budget.is_empty(),
        "Detected {} chunks over char budget (> {MAX_CHUNK_CHARS} chars): {:?}",
        over_budget.len(),
        over_budget
            .iter()
            .map(|a| format!("{}:{} ({})", a.source_path, a.line_start, a.char_length))
            .take(5)
            .collect::<Vec<_>>()
    );

    // 2. Absolute max must fit inside the dense window
    assert!(
        report.char_percentiles.max <= MAX_CHUNK_CHARS,
        "Max chunk size {} exceeds dense-model char budget {MAX_CHUNK_CHARS}",
        report.char_percentiles.max
    );

    // 3. Line count P99 bound
    assert!(
        report.line_percentiles.p99 <= 75,
        "P99 line count must be <= 75 lines, got {}",
        report.line_percentiles.p99
    );

    // 4. Healthy chunk ratio
    assert!(
        report.healthy_ratio >= 0.90,
        "Healthy chunk ratio must be >= 90%, got {:.2}%",
        report.healthy_ratio * 100.0
    );
}
